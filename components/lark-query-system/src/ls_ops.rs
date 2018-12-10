//! Contains "pseudo-queries" for language-server interaction. These
//! aren't *actual* queries, they are just functions, so they are not
//! memoized.  This also means they can take arbitrary parameters
//! (e.g. `&uri`) that wouldn't be possible otherwise, which is
//! convenient.

use languageserver_types::{Position, Range};
use lark_entity::{Entity, EntityData, ItemKind, MemberKind};
use lark_error::Diagnostic;
use lark_intern::{Intern, Untern};
use lark_parser::HoverTargetKind;
use lark_pretty_print::PrettyPrint;
use lark_span::{ByteIndex, FileName, IntoFileName, Span};
use std::collections::HashMap;

#[derive(Debug)]
pub struct RangedDiagnostic {
    pub label: String,
    pub range: Range,
}

impl RangedDiagnostic {
    pub fn new(label: String, range: Range) -> RangedDiagnostic {
        RangedDiagnostic { label, range }
    }
}

pub struct Cancelled;

pub type Cancelable<T> = Result<T, Cancelled>;

pub trait LsDatabase: lark_type_check::TypeCheckDatabase {
    fn check_for_cancellation(&self) -> Cancelable<()> {
        if self.salsa_runtime().is_current_revision_canceled() {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }

    fn errors_for_project(&self) -> Cancelable<HashMap<String, Vec<RangedDiagnostic>>> {
        let input_files = self.file_names();
        let mut file_errors = HashMap::new();

        for &input_file in &*input_files {
            self.check_for_cancellation()?;

            // Check file for syntax errors
            let mut errors = vec![];
            let _ = self
                .parsed_file(input_file)
                .accumulate_errors_into(&mut errors);

            // Next, check entities in file for type-safety
            let file_entity = EntityData::InputFile { file: input_file }.intern(self);
            for &entity in self.descendant_entities(file_entity).iter() {
                self.accumulate_errors_for_entity(entity, &mut errors)?;
            }

            let error_ranges = errors
                .iter()
                .map(|x| RangedDiagnostic::new(x.label.clone(), self.range(x.span)))
                .collect();

            file_errors.insert(input_file.id.untern(self).to_string(), error_ranges);
        }

        Ok(file_errors)
    }

    fn range(&self, span: Span<FileName>) -> languageserver_types::Range {
        let left = self.location(span.file(), span.start()).as_position();
        let right = self.location(span.file(), span.end()).as_position();
        languageserver_types::Range::new(left, right)
    }

    fn accumulate_errors_for_entity(
        &self,
        entity: Entity,
        errors: &mut Vec<Diagnostic>,
    ) -> Cancelable<()> {
        self.check_for_cancellation()?;

        match entity.untern(self) {
            EntityData::InputFile { .. } => {}
            EntityData::LangItem(_) => {}
            EntityData::Error(_) => {}
            EntityData::ItemName {
                kind: ItemKind::Struct,
                ..
            } => {
                let _ = self
                    .generic_declarations(entity)
                    .accumulate_errors_into(errors);
                let _ = self.ty(entity).accumulate_errors_into(errors);
            }
            EntityData::MemberName {
                kind: MemberKind::Field,
                ..
            } => {
                let _ = self
                    .generic_declarations(entity)
                    .accumulate_errors_into(errors);
                let _ = self.ty(entity).accumulate_errors_into(errors);
            }
            EntityData::ItemName {
                kind: ItemKind::Function,
                ..
            } => {
                let _ = self
                    .generic_declarations(entity)
                    .accumulate_errors_into(errors);
                let _ = self.ty(entity).accumulate_errors_into(errors);
                let _ = self.signature(entity).accumulate_errors_into(errors);
                let _ = self.fn_body(entity).accumulate_errors_into(errors);
                let _ = self.base_type_check(entity).accumulate_errors_into(errors);
            }
            EntityData::MemberName {
                kind: MemberKind::Method,
                ..
            } => {
                let _ = self
                    .generic_declarations(entity)
                    .accumulate_errors_into(errors);
                let _ = self.ty(entity).accumulate_errors_into(errors);
                let _ = self.signature(entity).accumulate_errors_into(errors);
                let _ = self.fn_body(entity).accumulate_errors_into(errors);
                let _ = self.base_type_check(entity).accumulate_errors_into(errors);
            }
        }

        Ok(())
    }

    fn definition_range_at_position(
        &self,
        url: &str,
        position: Position,
    ) -> Cancelable<Option<(String, Range)>> {
        let url_file_name = url.into_file_name(self);
        let byte_index = self.position_to_byte_index(url, position);
        let targets = self.hover_targets(url_file_name, byte_index);
        self.check_for_cancellation()?;

        Ok(targets
            .iter()
            .rev()
            .filter_map(|target| match target.kind {
                HoverTargetKind::MetaIndex(entity, mi) => match mi {
                    lark_hir::MetaIndex::Place(place_idx) => {
                        let fn_body = self.fn_body(entity).into_value();
                        let p = fn_body.tables[place_idx];
                        match p {
                            lark_hir::PlaceData::Entity(entity) => match entity.untern(self) {
                                EntityData::ItemName { .. } | EntityData::MemberName { .. } => {
                                    let span = self.parsed_entity(entity).full_span;
                                    let range = self.range(span);
                                    let filename = span.file().id.untern(self).to_string();
                                    Some((filename, range))
                                }
                                _ => None,
                            },
                            lark_hir::PlaceData::Variable(variable) => {
                                let span = fn_body.span(variable);
                                let range = self.range(span);
                                let filename = span.file().id.untern(self).to_string();
                                Some((filename, range))
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                },
                _ => None,
            })
            .next())
    }

    /// Returns the hover text to display for a given position (if
    /// any).
    fn hover_text_at_position(&self, url: &str, position: Position) -> Cancelable<Option<String>> {
        let url_file_name = url.into_file_name(self);
        let byte_index = self.position_to_byte_index(url, position);
        let targets = self.hover_targets(url_file_name, byte_index);
        self.check_for_cancellation()?;

        Ok(targets
            .iter()
            .rev()
            .filter_map(|target| match target.kind {
                HoverTargetKind::Entity(entity) => match entity.untern(self) {
                    EntityData::InputFile { .. }
                    | EntityData::LangItem(_)
                    | EntityData::Error(_) => None,
                    EntityData::ItemName {
                        kind: ItemKind::Struct,
                        ..
                    } => Some(format!("struct {}", entity.pretty_print(self))),
                    EntityData::ItemName {
                        kind: ItemKind::Function,
                        ..
                    } => Some(format!("def {}", entity.pretty_print(self))),
                    _ => Some(entity.pretty_print(self)),
                },

                HoverTargetKind::MetaIndex(entity, mi) => {
                    let fn_body_types = self.full_type_check(entity).into_value();
                    if let Some(ty) = fn_body_types.types.get(&mi) {
                        Some(format!("{}", ty.pretty_print(self)))
                    } else {
                        None
                    }
                }
            })
            .next())
    }

    fn position_to_byte_index(&self, url: &str, position: Position) -> ByteIndex {
        let url_id = url.intern(self);
        self.byte_index(FileName { id: url_id }, position.line, position.character)
    }
}
