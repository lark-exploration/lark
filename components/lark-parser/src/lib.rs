#![feature(const_fn)]
#![feature(const_let)]
#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(try_blocks)]
#![allow(dead_code)]

use crate::lexer::token::LexToken;
use crate::macros::EntityMacroDefinition;
use crate::syntax::entity::ParsedEntity;
use lark_collections::FxIndexMap;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::EntityTables;
use lark_entity::MemberKind;
use lark_error::Diagnostic;
use lark_error::ErrorReported;
use lark_error::WithError;
use lark_hir as hir;
use lark_intern::Intern;
use lark_seq::Seq;
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
mod fn_body;
mod ir;
mod lexer;
pub mod macros;
mod parser;
mod query_definitions;
mod scope;
pub mod syntax;
mod type_conversion;

pub use self::ir::ParsedFile;

salsa::query_group! {
    pub trait ParserDatabase: AsRef<GlobalIdentifierTables>
        + AsRef<EntityTables>
        + AsRef<DeclarationTables>
        + salsa::Database
    {
        fn file_names() -> Seq<FileName> {
            type FileNamesQuery;
            storage input;
        }

        fn file_text(id: FileName) -> Text {
            type FileTextQuery;
            storage input;
        }

        fn entity_span(entity: Entity) -> Span<FileName> {
            type EntitySpanQuery;
            use fn query_definitions::entity_span;
        }

        /// Returns, for each line in the given file, the start index
        /// -- the final element is the length of the file (there is
        /// kind of a "pseudo-empty line" at the end, so to speak). So
        /// for the input "a\nb\r\nc" you would get `[0, 2, 5, 6]`.
        fn line_offsets(id: FileName) -> Seq<usize> {
            type LineOffsetsQuery;
            use fn query_definitions::line_offsets;
        }

        fn location(id: FileName, index: ByteIndex) -> Location {
            type LocationQuery;
            use fn query_definitions::location;
        }

        /// Given a (zero-based) line number `line` and column within
        /// the line, gives a byte-index into the file's text.
        fn byte_index(id: FileName, line: u64, column: u64) -> ByteIndex {
            type ByteIndexQuery;
            use fn query_definitions::byte_index;
        }

        // FIXME: In general, this is wasteful of space, and not
        // esp. incremental friendly. It would be better store
        // e.g. the length of each token only, so that we can adjust
        // the previous value (not to mention perhaps using a rope or
        // some other similar data structure that permits insertions).
        fn file_tokens(id: FileName) -> WithError<Seq<Spanned<LexToken, FileName>>> {
            type FileTokensQuery;
            use fn query_definitions::file_tokens;
        }

        fn parsed_file(id: FileName) -> WithError<ParsedFile> {
            type ParsedFileQuery;
            use fn query_definitions::parsed_file;
        }

        fn child_parsed_entities(entity: Entity) -> WithError<Seq<ParsedEntity>> {
            type ChildParsedEntitiesQuery;
            use fn query_definitions::child_parsed_entities;
        }

        fn parsed_entity(entity: Entity) -> ParsedEntity {
            type ParsedEntityQuery;
            use fn query_definitions::parsed_entity;
        }

        /// Returns the immediate children of `entity` in the entity tree.
        fn child_entities(entity: Entity) -> Seq<Entity> {
            type ChildEntitiesQuery;
            use fn query_definitions::child_entities;
        }

        /// Transitive closure of `child_entities`.
        fn descendant_entities(entity: Entity) -> Seq<Entity> {
            type DescendantEntitiesQuery;
            use fn query_definitions::descendant_entities;
        }

        /// Get the fn-body for a given def-id.
        fn fn_body(key: Entity) -> WithError<Arc<hir::FnBody>> {
            type FnBodyQuery;
            use fn query_definitions::fn_body;
        }

        /// Get the list of member names and their def-ids for a given struct.
        fn members(key: Entity) -> Result<Seq<hir::Member>, ErrorReported> {
            type MembersQuery;
            use fn query_definitions::members;
        }

        /// Gets the def-id for a field of a given class.
        fn member_entity(entity: Entity, kind: MemberKind, id: GlobalIdentifier) -> Option<Entity> {
            type MemberEntityQuery;
            use fn query_definitions::member_entity;
        }

        /// Get the type of something.
        fn ty(key: Entity) -> WithError<ty::Ty<Declaration>> {
            type TyQuery;
            use fn type_conversion::ty;
        }

        /// Get the signature of a function.
        fn signature(key: Entity) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
            type SignatureQuery;
            use fn type_conversion::signature;
        }

        /// Get the generic declarations from a particular item.
        fn generic_declarations(key: Entity) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>> {
            type GenericDeclarationsQuery;
            use fn type_conversion::generic_declarations;
        }

        /// Resolve a type name that appears in the given entity.
        fn resolve_name(scope: Entity, name: GlobalIdentifier) -> Option<Entity> {
            type ResolveNameQuery;
            use fn scope::resolve_name;
        }
    }
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
