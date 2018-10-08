use crate::prelude::*;

use crate::parser::{ModuleTable, ParseError, Span, Spanned, StringId};
use crate::parser2::macros::Macro;
use crate::parser2::quicklex::Token as LexToken;

use codespan::CodeMap;
use std::collections::HashMap;

use derive_new::new;

#[derive(Debug, Copy, Clone)]
enum NextAction {
    Top,
    Macro(StringId),
}

#[derive(Debug, Copy, Clone)]
struct ScopeId {
    id: usize,
}

#[derive(Debug, Copy, Clone)]
struct BindingId {
    id: usize,
}

struct ReferenceId {
    scope_id: ScopeId,
    binding_id: BindingId,
}

#[derive(Debug)]
struct Scope {
    parent: ScopeId,
    bindings: Vec<BindingId>,
}

struct File {
    scopes: Vec<Scope>,
}

#[derive(Debug)]
struct AnnotatedToken {
    token: Token,
    scope_parent: ScopeId,
}

#[derive(Debug)]
enum Token {
    Binding(BindingId),
    Reference(BindingId),
    Label(StringId),
    Sigil(StringId),
    String(StringId),
}

enum TokenGroup {
    Single(Token),
    Grouped(GroupKind, Vec<Token>),
}

enum GroupKind {
    Brace,
    Paren,
}

struct TokenTree(Vec<TokenGroup>);

struct AnnotatedShape {
    tokens: Vec<AnnotatedToken>,
}

#[derive(Debug, new)]
struct LiteParser<'codemap> {
    tokens: Vec<Spanned<LexToken>>,
    macros: HashMap<StringId, Box<dyn Macro>>,
    table: ModuleTable,
    codemap: &'codemap CodeMap,

    #[new(value = "vec![]")]
    annotated: Vec<AnnotatedToken>,

    #[new(value = "0")]
    pos: usize,
}

impl LiteParser<'codemap> {
    pub fn process(mut self, scope: ScopeId) -> Result<TokenTree, ParseError> {
        while self.pos < self.tokens.len() {
            self.process_macro()?;
        }

        Ok(TokenTree(vec![]))
    }

    fn process_macro(&mut self) -> Result<(), ParseError> {
        match self.consume_leading_whitespace() {
            None => Err(ParseError::new(
                "Unexpected EOF in macro expansion, TODO".to_string(),
                Span::EOF,
            )),
            Some(token) => {
                println!("{:?}", token.as_id());
                let macro_def = self.macros.get(&token.as_id()?.node).ok_or_else(|| {
                    ParseError::new(
                        format!(
                            "No macro in scope {:?}",
                            Debuggable::from(&token.as_id().unwrap(), &self.table)
                        ),
                        token.span,
                    )
                });

                let terms = macro_def.annotate();

                Ok(())
            }
        }
    }

    fn consume(&mut self) -> Spanned<LexToken> {
        let token = self.tokens[self.pos];

        self.pos += 1;

        token
    }

    fn maybe_consume(&mut self) -> Option<Spanned<LexToken>> {
        if self.pos >= self.tokens.len() {
            None
        } else {
            Some(self.consume())
        }
    }

    /// In this context, leading whitespace includes newlines, because this
    /// method is used at the start of parse unit.
    fn consume_leading_whitespace(&mut self) -> Option<Spanned<LexToken>> {
        loop {
            let token = self.maybe_consume()?;

            println!("raw token={:?}", token);

            match token.node {
                LexToken::Whitespace(_) | LexToken::Newline => {}
                other => return Some(token),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LiteParser;

    use crate::parser::ast::DebuggableVec;
    use crate::parser::lexer_helpers::ParseError;
    use crate::parser::reporting::print_parse_error;
    use crate::parser::{Span, Spanned};
    use crate::parser2::quicklex::{Token, Tokenizer};
    use crate::parser2::test_helpers::{process, Annotations, Position};

    use log::trace;
    use std::collections::HashMap;
    use unindent::unindent;

    #[test]
    fn test_lite_parse() {
        crate::init_logger();

        let source = unindent(
            r##"
            struct Diagnostic {
            ^^^^^^~^^^^^^^^^^~^ @struct@ ws @Diagnostic@ ws #{#
              msg: own String,
              ^^^~^~~~^~~~~~~^ @msg@ #:# ws @own@ ws @String@ #,#
              level: String,
              ^^^^^~^~~~~~~^ @level@ #:# ws @String@ #,#
            }
            ^ #}#
            "##,
        );

        let (source, mut ann) = process(&source);

        let filemap = ann.codemap().add_filemap("test".into(), source.clone());
        let start = filemap.span().start().0;

        let tokens = match Tokenizer::new(ann.table(), &source, start).tokens() {
            Ok(tokens) => tokens,
            Err(e) => print_parse_error(e, ann.codemap()),
        };

        // let tokens: Result<Vec<Spanned<Token>>, ParseError> = lexed
        //     .map(|result| result.map(|(start, tok, end)| Spanned::from(tok, start, end)))
        //     .collect();

        println!("{:#?}", DebuggableVec::from(&tokens.clone(), ann.table()));

        let parser = LiteParser::new(tokens, HashMap::new(), ann.table().clone(), ann.codemap());

        match parser.process() {
            Ok(_) => {}
            Err(e) => print_parse_error(e, ann.codemap()),
        };
    }
}
