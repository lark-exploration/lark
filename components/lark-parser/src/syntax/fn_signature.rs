use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::entity::ErrorParsedEntity;
use crate::syntax::entity::LazyParsedEntity;
use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::syntax::fn_body;
use crate::syntax::guard::Guard;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::list::CommaList;
use crate::syntax::matched::Matched;
use crate::syntax::matched::ParsedMatch;
use crate::syntax::member::{Field, ParsedField};
use crate::syntax::sigil::{Curlies, Parentheses, RightArrow};
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::type_reference::ParsedTypeReference;
use crate::syntax::type_reference::TypeReference;
use crate::syntax::Syntax;
use lark_collections::Seq;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::ErrorReported;
use lark_error::ResultExt;
use lark_error::WithError;
use lark_hir as hir;
use lark_intern::Untern;
use lark_span::FileName;
use lark_span::Spanned;
use lark_string::GlobalIdentifier;
use lark_ty as ty;
use lark_ty::declaration::Declaration;

#[derive(DebugWith)]
pub struct FunctionSignature;

#[derive(Clone, DebugWith)]
pub struct ParsedFunctionSignature {
    pub parameters: Seq<Spanned<ParsedField, FileName>>,
    pub return_type: ParsedTypeReference,
    pub body: Result<Spanned<ParsedMatch, FileName>, ErrorReported>,
}

impl Syntax<'parse> for FunctionSignature {
    type Data = ParsedFunctionSignature;

    fn test(&mut self, parser: &Parser<'_>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&mut self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        let parameters = parser
            .expect(SkipNewline(Delimited(Parentheses, CommaList(Field))))
            .unwrap_or_else(|ErrorReported(_)| Seq::default());

        let return_type = match parser
            .parse_if_present(SkipNewline(Guard(RightArrow, SkipNewline(TypeReference))))
        {
            Some(ty) => ty.unwrap_or_error_sentinel(&*parser),
            None => ParsedTypeReference::Elided(parser.elided_span()),
        };

        let body = parser.expect(SkipNewline(Matched(Curlies)));

        Ok(ParsedFunctionSignature {
            parameters,
            return_type,
            body,
        })
    }
}

impl ParsedFunctionSignature {
    pub fn parse_signature(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
        self_ty: Option<ty::Ty<Declaration>>,
    ) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
        let mut errors = vec![];

        let inputs: Seq<_> = self_ty
            .into_iter()
            .chain(self.parameters.iter().map(|p| {
                p.ty.parse_type(entity, db)
                    .accumulate_errors_into(&mut errors)
            }))
            .collect();

        let output = self
            .return_type
            .parse_type(entity, db)
            .accumulate_errors_into(&mut errors);

        WithError {
            value: Ok(ty::Signature { inputs, output }),
            errors,
        }
    }

    pub fn parse_fn_body(
        &self,
        entity: Entity,
        db: &dyn LazyParsedEntityDatabase,
        self_argument: Option<Spanned<GlobalIdentifier, FileName>>,
    ) -> WithError<hir::FnBody> {
        match self.body {
            Err(err) => ErrorParsedEntity { err }.parse_fn_body(entity, db),

            Ok(Spanned {
                span: _,
                value:
                    ParsedMatch {
                        start_token,
                        end_token,
                    },
            }) => {
                let file_name = entity.untern(&db).file_name(&db).unwrap();
                let input = db.file_text(file_name);
                let tokens = db
                    .file_tokens(file_name)
                    .into_value()
                    .extract(start_token..end_token);
                let entity_macro_definitions = crate::macro_definitions(&db, entity);
                let arguments: Seq<_> = self.parameters.iter().map(|f| f.value.name).collect();
                fn_body::parse_fn_body(
                    entity,
                    db,
                    &entity_macro_definitions,
                    &input,
                    &tokens,
                    self_argument,
                    arguments,
                )
            }
        }
    }
}
