//! Contains "pseudo-queries" for language-server interaction. These
//! aren't *actual* queries, they are just functions, so they are not
//! memoized.  This also means they can take arbitrary parameters
//! (e.g. `&uri`) that wouldn't be possible otherwise, which is
//! convenient.

use codespan::{ByteIndex, ColumnIndex, FileMap, LineIndex};
use debug::DebugWith;
use intern::{Intern, Untern};
use languageserver_types::{Position, Range};
use lark_entity::{Entity, EntityData, ItemKind, MemberKind};
use lark_error::LabeledSpan;
use map::FxIndexMap;
use parking_lot::RwLock;
use parser::StringId;
use std::collections::HashMap;
use std::sync::Arc;

pub struct LabeledRange {
    pub label: String,
    pub range: Range,
}
impl LabeledRange {
    pub fn new(label: String, range: Range) -> LabeledRange {
        LabeledRange { label, range }
    }
}

pub struct Cancelled;

pub type Cancelable<T> = Result<T, Cancelled>;

pub trait LsDatabase: lark_type_check::TypeCheckDatabase {
    fn file_maps(&self) -> &RwLock<FxIndexMap<String, Arc<FileMap>>>;

    fn check_for_cancellation(&self) -> Cancelable<()> {
        if self.salsa_runtime().is_current_revision_canceled() {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }

    fn errors_for_project(&self) -> Cancelable<HashMap<String, Vec<LabeledRange>>> {
        let input_files = self.input_files(());
        let mut file_errors = HashMap::new();

        for input_file in &*input_files {
            self.check_for_cancellation()?;

            // Check file for syntax errors
            let mut errors = vec![];
            let _ = self
                .ast_of_file(*input_file)
                .accumulate_errors_into(&mut errors);

            // Next, check entities in file for type-safety
            let file_entity = EntityData::InputFile { file: *input_file }.intern(self);
            for &entity in self.subentities(file_entity).iter() {
                self.accumulate_errors_for_entity(entity, &mut errors)?;
            }

            let filename = self.untern_string(*input_file).to_string();
            let file_maps = self.file_maps().read().get(&filename).unwrap().clone();

            let error_ranges = errors
                .iter()
                .map(|x| {
                    let left_side = x.span.start().unwrap();
                    let (left_line, left_col) = file_maps.location(left_side).unwrap();
                    let left_position =
                        Position::new(left_line.to_usize() as u64, left_col.to_usize() as u64);

                    let right_side = x.span.end().unwrap();
                    let (right_line, right_col) = file_maps.location(right_side).unwrap();
                    let right_position =
                        Position::new(right_line.to_usize() as u64, right_col.to_usize() as u64);

                    LabeledRange::new(x.label.clone(), Range::new(left_position, right_position))
                })
                .collect();

            file_errors.insert(filename, error_ranges);
        }

        Ok(file_errors)
    }

    fn accumulate_errors_for_entity(
        &self,
        entity: Entity,
        errors: &mut Vec<LabeledSpan>,
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
        let interned_path = self.intern_string(url);
        let entity_ids = self.entity_ids_at_position(interned_path, byte_index)?;
        self.check_for_cancellation()?;
        let entity = *entity_ids.last().unwrap();
        match entity.untern(self) {
            EntityData::ItemName {
                kind: ItemKind::Struct,
                id,
                ..
            } => Ok(Some(format!("struct {}", self.untern_string(id)))),

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
        let file_maps = self.file_maps().read();
        file_maps[url]
            .byte_index(
                LineIndex(position.line as u32),
                ColumnIndex(position.character as u32),
            )
            .unwrap()
    }

    /// Return a "stack" of entity-ids in position, from outermost to
    /// innermost.  Always returns a non-empty vector.
    fn entity_ids_at_position(
        &self,
        path: StringId,
        position: ByteIndex,
    ) -> Cancelable<Vec<Entity>> {
        self.check_for_cancellation()?;

        let file_entity = EntityData::InputFile { file: path }.intern(self);

        let mut entities: Vec<_> = self
            .subentities(file_entity)
            .iter()
            .filter_map(|&entity| {
                if let Some(span) = self.entity_span(entity) {
                    if span.contains(position) {
                        return Some(entity);
                    }
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
            let span = self.entity_span(entity).unwrap();
            let start = span.start().map(|v| v.to_usize());
            let end = span.end().map(|v| std::usize::MAX - v.to_usize());
            (start, end)
        });

        assert!(!entities.is_empty());
        Ok(entities)
    }
}
