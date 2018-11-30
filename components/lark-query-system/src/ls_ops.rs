//! Contains "pseudo-queries" for language-server interaction. These
//! aren't *actual* queries, they are just functions, so they are not
//! memoized.  This also means they can take arbitrary parameters
//! (e.g. `&uri`) that wouldn't be possible otherwise, which is
//! convenient.

use languageserver_types::{Position, Range};
use lark_debug_with::DebugWith;
use lark_entity::{Entity, EntityData, ItemKind, MemberKind};
use lark_error::Diagnostic;
use lark_intern::{Intern, Untern};
use lark_span::{ByteIndex, FileName, IntoFileName};
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

            let text = self.file_text(input_file);

            let error_ranges = errors
                .iter()
                .map(|x| RangedDiagnostic::new(x.label.clone(), x.span.to_range(&text).unwrap()))
                .collect();

            file_errors.insert(input_file.id.untern(self).to_string(), error_ranges);
        }

        Ok(file_errors)
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

    /// Returns the hover text to display for a given position (if
    /// any).
    fn hover_text_at_position(&self, url: &str, position: Position) -> Cancelable<Option<String>> {
        let byte_index = self.position_to_byte_index(url, position);
        let entity_ids = self.entity_ids_at_position(url, byte_index)?;
        self.check_for_cancellation()?;
        let entity = *entity_ids.last().unwrap();
        match entity.untern(self) {
            EntityData::ItemName {
                kind: ItemKind::Struct,
                id,
                ..
            } => Ok(Some(format!("struct {}", id.untern(self)))),

            EntityData::MemberName {
                kind: MemberKind::Field,
                ..
            } => {
                let field_ty = self.ty(entity).into_value();
                // FIXME should not use "debug" but display to format the type
                Ok(Some(format!("{}", field_ty.debug_with(self))))
            }

            EntityData::ItemName {
                kind: ItemKind::Function,
                ..
            }
            | EntityData::MemberName {
                kind: MemberKind::Method,
                ..
            } => {
                // what should we say for functions and methods?
                Ok(None)
            }

            EntityData::InputFile { .. } | EntityData::LangItem(_) | EntityData::Error(_) => {
                Ok(None)
            }
        }
    }

    fn position_to_byte_index(&self, url: &str, position: Position) -> ByteIndex {
        let url_id = url.intern(self);
        self.byte_index(FileName { id: url_id }, position.line, position.character)
    }

    /// Return a "stack" of entity-ids in position, from outermost to
    /// innermost.  Always returns a non-empty vector.
    fn entity_ids_at_position(
        &self,
        file: impl IntoFileName,
        index: ByteIndex,
    ) -> Cancelable<Vec<Entity>> {
        let file = file.into_file_name(self);

        self.check_for_cancellation()?;

        let file_entity = EntityData::InputFile { file }.intern(self);

        let mut entities: Vec<_> = self
            .descendant_entities(file_entity)
            .iter()
            .filter_map(|&entity| {
                let span = self.entity_span(entity);
                if span.contains_index(index) {
                    return Some(entity);
                }

                None
            })
            .collect();

        // If we assume that all the entities contain one another,
        // then sorting by their *start spans* first (and inversely by
        // *end spans* in case of ties...)  should give in
        // "outermost-to-innermost" order.
        //
        // Example:
        //
        // foo { bar { } }
        //       ^^^       2
        //       ^^^^^^^   1
        // ^^^^^^^^^^^^^^^ 0
        entities.sort_by_key(|&entity| {
            let span = self.entity_span(entity);
            let start = span.start();
            let end = std::usize::MAX - span.end().to_usize();
            (start, end)
        });

        assert!(!entities.is_empty());
        Ok(entities)
    }
}
