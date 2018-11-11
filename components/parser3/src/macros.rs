use crate::parsed_entity::ParsedEntity;
use crate::parser::Parser;
use crate::span::Spanned;
use intern::Intern;
use lark_entity::Entity;
use lark_string::global::GlobalIdentifier;
use lark_string::global::GlobalIdentifierTables;
use map::FxIndexMap;
use std::sync::Arc;

macro_rules! or_error_entity {
    ($v:expr, $parser:expr, $message:expr) => {
        match $v {
            Some(v) => v,
            None => {
                let current_token_span = $parser.peek_span();
                return $parser.error_entity($message, current_token_span);
            }
        }
    };
}

crate mod struct_declaration;

crate trait EntityMacroDefinition {
    /// Invoked when the macro name has been recognized and
    /// consumed. Has the job of parsing the rest of the entity (using
    /// the helper methods on `parser` to do so) and ultimately
    /// returning the entity structure.
    fn parse(
        &self,
        // The parser we can use to extract next token and so forth.
        parser: &mut Parser<'_>,

        // The base entity that this is a subentity of. Needed to
        // create a `lark_entity::Entity`.
        base: Entity,

        // The macro name we were invoked with (and the span). Note
        // that the "start" of this span will also be the start of
        // our entity's span.
        macro_name: Spanned<GlobalIdentifier>,
    ) -> ParsedEntity;
}

macro_rules! declare_macro {
    (
        db($db:expr),
        macros($($name:expr => $macro_definition:ty,)*),
    ) => {
        {
            let mut map: FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>> = FxIndexMap::default();
            $(
                let name = $name.intern($db);
                map.insert(name, std::sync::Arc::new(<$macro_definition>::default()));
            )*
                map
        }
    }
}

crate fn default_entity_macros(
    db: &dyn AsRef<GlobalIdentifierTables>,
) -> FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>> {
    declare_macro!(
        db(db),
        macros(
            "struct" => struct_declaration::StructDeclaration,
        ),
    )
}