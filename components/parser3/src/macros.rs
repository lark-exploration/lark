use crate::parsed_entity::ParsedEntity;
use crate::parser::Parser;
use crate::span::CurrentFile;
use crate::span::Location;
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
crate mod type_reference;

crate trait EntityMacroDefinition {
    fn parse(
        &self,
        parser: &mut Parser<'_>,
        base: Entity,
        start: Location<CurrentFile>,
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
