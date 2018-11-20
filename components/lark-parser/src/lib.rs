#![feature(const_fn)]
#![feature(const_let)]
#![feature(crate_visibility_modifier)]
#![feature(macro_at_most_once_rep)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(try_blocks)]
#![allow(dead_code)]

use crate::lexer::token::LexToken;

use intern::Intern;
use lark_debug_derive::DebugWith;
use lark_entity::{Entity, EntityTables, MemberKind};
use lark_error::{Diagnostic, ErrorReported, WithError};
use lark_hir::FnBody;
use lark_hir::Member;
use lark_seq::Seq;
use lark_span::{ByteIndex, FileName, Location, Span, Spanned};
use lark_string::{GlobalIdentifier, GlobalIdentifierTables, Text};
use lark_ty as ty;
use lark_ty::declaration::{Declaration, DeclarationTables};
use std::sync::Arc;

pub mod current_file;
mod fn_body;
mod ir;
mod lexer;
mod macros;
mod parser;
mod query_definitions;
mod scope;
pub mod syntax;
mod type_conversion;

pub use self::ir::ParsedFile;
pub use self::syntax::entity::ParsedEntity;
pub use self::syntax::uhir;

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

        fn parsed_entity(entity: Entity) -> WithError<ParsedEntity> {
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
        fn fn_body(key: Entity) -> WithError<Arc<FnBody>> {
            type FnBodyQuery;
            use fn fn_body::fn_body;
        }

        /// Get the list of member names and their def-ids for a given struct.
        fn members(key: Entity) -> Result<Seq<Member>, ErrorReported> {
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

pub trait IntoFileName {
    fn into_file_name(&self, db: &impl ParserDatabase) -> FileName;
}

impl IntoFileName for FileName {
    fn into_file_name(&self, _db: &impl ParserDatabase) -> FileName {
        *self
    }
}

impl IntoFileName for &str {
    fn into_file_name(&self, db: &impl ParserDatabase) -> FileName {
        FileName {
            id: self.intern(db),
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
}

fn diagnostic(message: impl Into<String>, span: Span<FileName>) -> Diagnostic {
    Diagnostic::new(message.into(), span)
}
