//! Contains "pseudo-queries" for language-server interaction. These
//! aren't *actual* queries, they are just functions, so they are not
//! memoized.  This also means they can take arbitrary parameters
//! (e.g. `&uri`) that wouldn't be possible otherwise, which is
//! convenient.

use codespan::{ByteIndex, ColumnIndex, FileMap, LineIndex};
use intern::Intern;
use languageserver_types::Position;
use lark_entity::{Entity, EntityData};
use map::FxIndexMap;
use parking_lot::RwLock;
use parser::StringId;
use std::sync::Arc;

pub(crate) struct Cancelled;

pub(crate) type Cancelable<T> = Result<T, Cancelled>;

pub(crate) trait LsDatabase: type_check::TypeCheckDatabase {
    fn file_maps(&self) -> &RwLock<FxIndexMap<String, Arc<FileMap>>>;

    fn check_for_cancellation(&self) -> Cancelable<()> {
        if self.salsa_runtime().is_current_revision_canceled() {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }

    fn type_at_position(&self, url: &str, position: Position) -> Cancelable<String> {
        let byte_index = self.position_to_byte_index(url, position);
        let interned_path = self.intern_string(url);
        let entity_ids = self.entity_ids_at_position(interned_path, byte_index)?;
        Ok(format!("{:?}", entity_ids))
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

        Ok(entities)
    }
}
