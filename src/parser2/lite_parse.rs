use crate::parser::{Spanned, StringId};
use crate::parser2::macros::Macro;
use crate::parser2::quicklex::Token as LexToken;

use std::collections::HashMap;

use derive_new::new;

#[derive(Debug, new)]
struct LiteParser {
    tokens: Vec<Spanned<LexToken>>,
    macros: HashMap<StringId, Box<dyn Macro>>,

    #[new(value = "0")]
    pos: usize,
}

enum NextAction {
    Top,
    Macro(StringId),
}

struct ScopeId {
    id: usize,
}

struct BindingId {
    id: usize,
}

struct Scope {
    parent: ScopeId,
    bindings: Vec<BindingId>,
}

struct File {
    scopes: Vec<Scope>,
}

struct AnnotatedToken {
    token: Token,
    scope_parent: ScopeId,
}

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

impl LiteParser {
    pub fn process(mut self) -> TokenTree {
        loop {
            self.process_macro();
        }
    }

    fn process_macro(&mut self) {
        let token = self.consume().as_id();
    }

    fn consume(&mut self) -> Spanned<LexToken> {
        let token = self.tokens[self.pos];

        self.pos += 1;

        token
    }
}

#[cfg(test)]
mod tests {
    use super::LiteParser;

    use crate::parser::ast::DebuggableVec;
    use crate::parser::lexer_helpers::ParseError;
    use crate::parser::{Span, Spanned};
    use crate::parser2::quicklex::{Token, Tokenizer};
    use crate::parser2::test_helpers::{process, Annotations, Position};

    use log::trace;
    use unindent::unindent;

    #[test]
    fn test_lite_parse() -> Result<(), ParseError> {
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

        let tokens = Tokenizer::new(ann.table(), &source, start).tokens();

        // let tokens: Result<Vec<Spanned<Token>>, ParseError> = lexed
        //     .map(|result| result.map(|(start, tok, end)| Spanned::from(tok, start, end)))
        //     .collect();

        println!("{:#?}", DebuggableVec::from(&tokens.clone()?, ann.table()));

        let parser = LiteParser::new(tokens?, HashMap::new());

        parser.process();

        Ok(())
    }
}
