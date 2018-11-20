use crate::parser::Parser;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::Syntax;
use intern::Intern;
use intern::Untern;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_error::{ErrorReported, ErrorSentinel, WithError};
use lark_span::{FileName, Span, Spanned};
use lark_string::GlobalIdentifier;
use lark_ty as ty;
use lark_ty::declaration::Declaration;
use lark_ty::TypeFamily;

#[derive(DebugWith)]
pub struct TypeReference;

impl Syntax<'parse> for TypeReference {
    type Data = ParsedTypeReference;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(
        &mut self,
        parser: &mut Parser<'parse>,
    ) -> Result<ParsedTypeReference, ErrorReported> {
        let identifier = parser.expect(SpannedGlobalIdentifier)?;
        Ok(ParsedTypeReference::Named(NamedTypeReference {
            identifier,
        }))
    }
}

/// Parsed form of a type.
#[derive(Copy, Clone, DebugWith)]
pub enum ParsedTypeReference {
    Named(NamedTypeReference),
    Elided(Span<FileName>),
    Error,
}

impl ParsedTypeReference {
    pub fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<ty::Ty<Declaration>> {
        match self {
            ParsedTypeReference::Named(named) => named.parse_type(entity, db),
            ParsedTypeReference::Elided(_span) => {
                let entity = EntityData::LangItem(LangItem::Tuple(0)).intern(&db);
                WithError::ok(ty::Ty {
                    perm: ty::Erased,
                    base: Declaration::intern_base_data(
                        &db,
                        ty::BaseData {
                            kind: ty::BaseKind::Named(entity),
                            generics: ty::Generics::empty(),
                        },
                    ),
                })
            }
            ParsedTypeReference::Error => WithError::ok(Declaration::error_type(&db)),
        }
    }
}

impl<Cx> ErrorSentinel<Cx> for ParsedTypeReference {
    fn error_sentinel(_cx: Cx, _report: ErrorReported) -> Self {
        ParsedTypeReference::Error
    }
}

/// Named type like `String` or (eventually) `Vec<u32>`
#[derive(Copy, Clone, DebugWith)]
pub struct NamedTypeReference {
    pub identifier: Spanned<GlobalIdentifier, FileName>,
}

impl NamedTypeReference {
    pub fn parse_type(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
    ) -> WithError<ty::Ty<Declaration>> {
        match db.resolve_name(entity, self.identifier.value) {
            Some(entity) => {
                let ty = crate::type_conversion::declaration_ty_named(
                    &db,
                    entity,
                    ty::Generics::empty(),
                );
                WithError::ok(ty)
            }
            None => {
                let msg = format!("unknown type: `{}`", self.identifier.untern(&db));
                WithError::report_error(&db, msg, self.identifier.span)
            }
        }
    }
}
