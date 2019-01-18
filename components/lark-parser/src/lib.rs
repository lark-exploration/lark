#![feature(const_fn)]
#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(try_blocks)]
#![allow(dead_code)]

use crate::lexer::token::LexToken;
use crate::macros::EntityMacroDefinition;
use crate::syntax::entity::ParsedEntity;
use lark_collections::{FxIndexMap, Seq};
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::EntityTables;
use lark_entity::MemberKind;
use lark_error::Diagnostic;
use lark_error::ErrorReported;
use lark_error::WithError;
use lark_hir as hir;
use lark_intern::Intern;
use lark_span::ByteIndex;
use lark_span::FileName;
use lark_span::IntoFileName;
use lark_span::Location;
use lark_span::Span;
use lark_span::Spanned;
use lark_string::GlobalIdentifier;
use lark_string::GlobalIdentifierTables;
use lark_string::Text;
use lark_ty as ty;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclarationTables;
use std::sync::Arc;

pub mod current_file;
mod ir;
mod lexer;
pub mod macros;
mod parser;
mod query_definitions;
mod scope;
pub mod syntax;
mod type_conversion;

pub use self::ir::ParsedFile;

#[salsa::query_group]
pub trait ParserDatabase:
    AsRef<GlobalIdentifierTables> + AsRef<EntityTables> + AsRef<DeclarationTables> + salsa::Database
{
    #[salsa::input]
    fn file_names(&self) -> Seq<FileName>;

    #[salsa::input]
    fn file_text(&self, id: FileName) -> Text;

    #[salsa::invoke(query_definitions::entity_span)]
    fn entity_span(&self, entity: Entity) -> Span<FileName>;

    #[salsa::invoke(query_definitions::characteristic_entity_span)]
    fn characteristic_entity_span(&self, entity: Entity) -> Span<FileName>;

    /// Returns, for each line in the given file, the start index
    /// -- the final element is the length of the file (there is
    /// kind of a "pseudo-empty line" at the end, so to speak). So
    /// for the input "a\nb\r\nc" you would get `[0, 2, 5, 6]`.
    #[salsa::invoke(query_definitions::line_offsets)]
    fn line_offsets(&self, id: FileName) -> Seq<usize>;

    #[salsa::invoke(query_definitions::location)]
    fn location(&self, id: FileName, index: ByteIndex) -> Location;

    /// Given a (zero-based) line number `line` and column within
    /// the line, gives a byte-index into the file's text.
    #[salsa::invoke(query_definitions::byte_index)]
    fn byte_index(&self, id: FileName, line: u64, column: u64) -> ByteIndex;

    // FIXME: In general, this is wasteful of space, and not
    // esp. incremental friendly. It would be better store
    // e.g. the length of each token only, so that we can adjust
    // the previous value (not to mention perhaps using a rope or
    // some other similar data structure that permits insertions).
    #[salsa::invoke(query_definitions::file_tokens)]
    fn file_tokens(&self, id: FileName) -> WithError<Seq<Spanned<LexToken, FileName>>>;

    #[salsa::invoke(query_definitions::parsed_file)]
    fn parsed_file(&self, id: FileName) -> WithError<ParsedFile>;

    #[salsa::invoke(query_definitions::child_parsed_entities)]
    fn child_parsed_entities(&self, entity: Entity) -> WithError<Seq<ParsedEntity>>;

    #[salsa::invoke(query_definitions::parsed_entity)]
    fn parsed_entity(&self, entity: Entity) -> ParsedEntity;

    /// Returns the immediate children of `entity` in the entity tree.
    #[salsa::invoke(query_definitions::child_entities)]
    fn child_entities(&self, entity: Entity) -> Seq<Entity>;

    /// Transitive closure of `child_entities`.
    #[salsa::invoke(query_definitions::descendant_entities)]
    fn descendant_entities(&self, entity: Entity) -> Seq<Entity>;

    /// Get the fn-body for a given def-id.
    #[salsa::invoke(query_definitions::fn_body)]
    fn fn_body(&self, key: Entity) -> WithError<Arc<hir::FnBody>>;

    /// Given a span, find the things that it may have been referring to.
    #[salsa::invoke(query_definitions::hover_targets)]
    fn hover_targets(&self, file: FileName, index: ByteIndex) -> Seq<HoverTarget>;

    /// Get the list of member names and their def-ids for a given struct.
    #[salsa::invoke(query_definitions::members)]
    fn members(&self, key: Entity) -> Result<Seq<hir::Member>, ErrorReported>;

    /// Gets the def-id for a field of a given class.
    #[salsa::invoke(query_definitions::member_entity)]
    fn member_entity(
        &self,
        entity: Entity,
        kind: MemberKind,
        id: GlobalIdentifier,
    ) -> Option<Entity>;

    /// Get the type of something.
    #[salsa::invoke(type_conversion::ty)]
    fn ty(&self, key: Entity) -> WithError<ty::Ty<Declaration>>;

    /// Get the signature of a function.
    #[salsa::invoke(type_conversion::signature)]
    fn signature(
        &self,
        key: Entity,
    ) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>>;

    /// Get the generic declarations from a particular item.
    #[salsa::invoke(type_conversion::generic_declarations)]
    fn generic_declarations(
        &self,
        key: Entity,
    ) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>>;

    /// Resolve a type name that appears in the given entity.
    #[salsa::invoke(scope::resolve_name)]
    fn resolve_name(&self, scope: Entity, name: GlobalIdentifier) -> Option<Entity>;
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq)]
pub struct HoverTarget {
    pub span: Span<FileName>,
    pub kind: HoverTargetKind,
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq)]
pub enum HoverTargetKind {
    Entity(Entity),
    MetaIndex(Entity, hir::MetaIndex),
}

pub trait ParserDatabaseExt: ParserDatabase {
    fn init_parser_db(&mut self) {
        self.query_mut(FileNamesQuery).set((), Default::default());
    }

    fn add_file(&mut self, path: impl IntoFileName, contents: impl Into<Text>) {
        let file_name = path.into_file_name(self);

        let mut file_names = self.file_names();
        file_names.extend(Some(file_name));

        self.query_mut(FileNamesQuery).set((), file_names);
        self.query_mut(FileTextQuery)
            .set(file_name, contents.into());
    }

    /// Returns the "top-level" entities defined in the given file --
    /// does not descend to visit the children of those entities etc.
    fn top_level_entities_in_file(&self, file: impl IntoFileName) -> Seq<Entity> {
        let file = file.into_file_name(self);
        let file_entity = EntityData::InputFile { file }.intern(self);
        self.child_entities(file_entity)
    }
}

fn diagnostic(message: impl Into<String>, span: Span<FileName>) -> Diagnostic {
    Diagnostic::new(message.into(), span)
}

/// Set of macro definitions in scope for `entity`. For now, this is
/// always the default set. This function really just exists as a
/// placeholder for us to change later.
fn macro_definitions(
    db: &dyn AsRef<GlobalIdentifierTables>,
    _entity: Entity,
) -> FxIndexMap<GlobalIdentifier, Arc<dyn EntityMacroDefinition>> {
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

    declare_macro!(
        db(db),
        macros(
            "struct" => macros::struct_declaration::StructDeclaration,
            "def" => macros::function_declaration::FunctionDeclaration,
        ),
    )
}
