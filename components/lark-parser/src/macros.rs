use crate::parser::Parser;
use crate::syntax::entity::ParsedEntity;

use intern::Intern;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_span::{FileName, Spanned};
use lark_string::{GlobalIdentifier, GlobalIdentifierTables};
use map::FxIndexMap;
use std::sync::Arc;

crate mod function_declaration;
crate mod struct_declaration;

crate trait EntityMacroDefinition {
    /// Invoked when the macro name has been recognized and
    /// consumed. Has the job of parsing the rest of the entity (using
    /// the helper methods on `parser` to do so) and ultimately
    /// returning the entity structure.
    fn expect(
        &self,
        // The parser we can use to extract next token and so forth.
        parser: &mut Parser<'_>,

        // The base entity that this is a subentity of. Needed to
        // create a `lark_entity::Entity`.
        base: Entity,

        // The macro name we were invoked with (and the span). Note
        // that the "start" of this span will also be the start of
        // our entity's span.
        macro_name: Spanned<GlobalIdentifier, FileName>,
    ) -> Result<ParsedEntity, ErrorReported>;
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
            "fn" => function_declaration::FunctionDeclaration,
        ),
    )
}
