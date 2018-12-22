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
                let _ = self.full_type_check(entity).accumulate_errors_into(errors);
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
                let _ = self.full_type_check(entity).accumulate_errors_into(errors);
            }
        }

        Ok(())
    }

    fn find_all_references_to_definition(&self, definition_entity: Entity) -> Vec<(String, Range)> {
        let input_files = self.file_names();
        let mut uses = vec![];

        let p = lark_hir::PlaceData::Entity(definition_entity);

        for &input_file in &*input_files {
            let _ = self.parsed_file(input_file);

            let file_entity = EntityData::InputFile { file: input_file }.intern(self);
            for &entity in self.descendant_entities(file_entity).iter() {
                if entity.untern(self).has_fn_body() {
                    let fn_body = self.fn_body(entity).into_value();
                    for (key, value) in fn_body.tables.places.iter_enumerated() {
                        if *value == p {
                            let span = fn_body.span(key);
                            let range = self.range(span);
                            let filename = span.file().id.untern(self).to_string();
                            uses.push((filename, range));
                        }
                    }
                }
            }
        }

        uses
    }

    fn find_all_references_to_variable(
        &self,
        fn_body: &lark_hir::FnBody,
        variable: lark_hir::Variable,
    ) -> Vec<(String, Range)> {
        let mut uses = vec![];

        let p = lark_hir::PlaceData::Variable(variable);

        for (key, value) in fn_body.tables.places.iter_enumerated() {
            if *value == p {
                let span = fn_body.span(key);
                let range = self.range(span);
                let filename = span.file().id.untern(self).to_string();
                uses.push((filename, range));
            }
        }

        uses
    }

    fn find_all_references_to_field(&self, field_entity: Entity) -> Vec<(String, Range)> {
        let input_files = self.file_names();
        let mut uses = vec![];

        for &input_file in &*input_files {
            let _ = self.parsed_file(input_file);

            let file_entity = EntityData::InputFile { file: input_file }.intern(self);
            for &entity in self.descendant_entities(file_entity).iter() {
                if entity.untern(self).has_fn_body() {
                    let fn_body = self.fn_body(entity).into_value();
                    let possible_match_types = &self.full_type_check(entity).into_value();

                    for value in fn_body.tables.places.iter() {
                        match value {
                            lark_hir::PlaceData::Field {
                                name: value_name, ..
                            } => {
                                if possible_match_types.entities[&(*value_name).into()]
                                    == field_entity
                                {
                                    let span = fn_body.span(*value_name);
                                    let range = self.range(span);
                                    let filename = span.file().id.untern(self).to_string();
                                    uses.push((filename, range));
                                }
                            }
                            _ => {}
                        }
                    }

                    for identified_expression in fn_body.tables.identified_expressions.iter() {
                        match &identified_expression {
                            lark_hir::IdentifiedExpressionData { identifier, .. } => {
                                if possible_match_types.entities[&(*identifier).into()]
                                    == field_entity
                                {
                                    let span = fn_body.span(*identifier);
                                    let range = self.range(span);
                                    let filename = span.file().id.untern(self).to_string();
                                    uses.push((filename, range));
                                }
                            }
                        }
                    }
                }
            }
        }

        uses
    }

    fn rename_all_references_at_position(
        &self,
        url: &str,
        position: Position,
        new_name: &str,
    ) -> Cancelable<Vec<(String, Range, String)>> {
        self.check_for_cancellation()?;

        let references = self.find_all_references_at_position(url, position)?;

        Ok(references
            .into_iter()
            .map(|(x, y)| (x, y, new_name.to_string()))
            .collect())
    }

    fn find_all_references_at_position(
        &self,
        url: &str,
        position: Position,
    ) -> Cancelable<Vec<(String, Range)>> {
        // First, let's add the definition site, as this is one of the references
        let definition_position = self.definition_range_at_position(url, position, true)?;

        // Then, we gather the uses
        let url_file_name = url.into_file_name(self);
        let byte_index = self.position_to_byte_index(url, position);
        let targets = self.hover_targets(url_file_name, byte_index);
        self.check_for_cancellation()?;

        let results = targets
            .iter()
            .rev()
            .filter_map(|target| match target.kind {
                HoverTargetKind::Entity(hovered_entity) => match hovered_entity.untern(self) {
                    EntityData::MemberName {
                        kind: MemberKind::Field,
                        ..
                    } => Some(self.find_all_references_to_field(hovered_entity)),
                    _ => Some(self.find_all_references_to_definition(hovered_entity)),
                },
                HoverTargetKind::MetaIndex(entity, mi) => match mi {
                    lark_hir::MetaIndex::Variable(variable) => {
                        let fn_body = self.fn_body(entity).into_value();
                        Some(self.find_all_references_to_variable(&fn_body, variable))
                    }
                    lark_hir::MetaIndex::Place(place_idx) => {
                        let fn_body = self.fn_body(entity).into_value();
                        let p = fn_body.tables[place_idx];

                        match p {
                            lark_hir::PlaceData::Entity(entity) => {
                                Some(self.find_all_references_to_definition(entity))
                            }
                            lark_hir::PlaceData::Variable(variable) => {
                                Some(self.find_all_references_to_variable(&fn_body, variable))
                            }
                            lark_hir::PlaceData::Field { name, .. } => {
                                let source_types = &self.full_type_check(entity).into_value();
                                let hovered_entity = source_types.entities[&name.into()];

                                Some(self.find_all_references_to_field(hovered_entity))
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                },
            })
            .next();

        if results.is_some() {
            let mut results = results.unwrap();

            if definition_position.is_some() {
                results.push(definition_position.unwrap());
            }

            Ok(results)
        } else {
            if definition_position.is_some() {
                Ok(vec![definition_position.unwrap()])
            } else {
                Ok(vec![])
            }
        }
    }

    fn get_entity_span_if_possible(
        &self,
        entity: Entity,
        use_minimal_span: bool,
    ) -> Option<Span<FileName>> {
        match entity.untern(self) {
            EntityData::Error(..) | EntityData::LangItem { .. } => None,
            _ => {
                if use_minimal_span {
                    Some(self.characteristic_entity_span(entity))
                } else {
                    Some(self.entity_span(entity))
                }
            }
        }
    }

    fn definition_range_at_position(
        &self,
        url: &str,
        position: Position,
        minimal_span: bool,
    ) -> Cancelable<Option<(String, Range)>> {
        let url_file_name = url.into_file_name(self);
        let byte_index = self.position_to_byte_index(url, position);
        let targets = self.hover_targets(url_file_name, byte_index);
        self.check_for_cancellation()?;

        Ok(targets
            .iter()
            .rev()
            .filter_map(|target| match target.kind {
                HoverTargetKind::Entity(entity) => {
                    if let Some(span) = self.get_entity_span_if_possible(entity, minimal_span) {
                        let range = self.range(span);
                        let filename = span.file().id.untern(self).to_string();
                        Some((filename, range))
                    } else {
                        None
                    }
                }
                HoverTargetKind::MetaIndex(entity, mi) => match mi {
                    lark_hir::MetaIndex::Identifier(identifier) => {
                        let source_types = &self.full_type_check(entity).into_value();
                        if let Some(target_entity) = source_types.entities.get(&identifier.into()) {
                            if let Some(span) =
                                self.get_entity_span_if_possible(*target_entity, minimal_span)
                            {
                                let range = self.range(span);
                                let filename = span.file().id.untern(self).to_string();
                                Some((filename, range))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    lark_hir::MetaIndex::Variable(variable) => {
                        let fn_body = self.fn_body(entity).into_value();
                        let span = fn_body.span(variable);
                        let range = self.range(span);
                        let filename = span.file().id.untern(self).to_string();
                        Some((filename, range))
                    }
                    lark_hir::MetaIndex::Place(place_idx) => {
                        let fn_body = self.fn_body(entity).into_value();
                        let p = fn_body.tables[place_idx];

                        match p {
                            lark_hir::PlaceData::Entity(entity) => {
                                if let Some(span) =
                                    self.get_entity_span_if_possible(entity, minimal_span)
                                {
                                    let range = self.range(span);
                                    let filename = span.file().id.untern(self).to_string();
                                    Some((filename, range))
                                } else {
                                    let span = fn_body.span(place_idx);
                                    let range = self.range(span);
                                    let filename = span.file().id.untern(self).to_string();
                                    Some((filename, range))
                                }
                            }
                            lark_hir::PlaceData::Variable(variable) => {
                                let span = fn_body.span(variable);
                                let range = self.range(span);
                                let filename = span.file().id.untern(self).to_string();
                                Some((filename, range))
                            }
                            lark_hir::PlaceData::Field { name, .. } => {
                                let results = &self.full_type_check(entity).into_value();

                                match results.entities.get(&name.into()) {
                                    Some(child_entity) => {
                                        if let Some(span) = self.get_entity_span_if_possible(
                                            *child_entity,
                                            minimal_span,
                                        ) {
                                            let range = self.range(span);
                                            let filename = span.file().id.untern(self).to_string();
                                            Some((filename, range))
                                        } else {
                                            None
                                        }
                                    }

                                    None => None,
                                }
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                },
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

                    match mi {
                        lark_hir::MetaIndex::Identifier(identifier) => {
                            if let Some(target_entity) =
                                fn_body_types.entities.get(&identifier.into())
                            {
                                Some(format!(
                                    "{}",
                                    self.ty(*target_entity).value.pretty_print(self),
                                ))
                            } else {
                                None
                            }
                        }
                        _ => {
                            if let Some(ty) = fn_body_types.opt_ty(mi) {
                                Some(format!("{}", ty.pretty_print(self),))
                            } else {
                                None
                            }
                        }
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
