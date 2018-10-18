use crate::prelude::*;

use crate::parser::{ModuleTable, ParseError, Span, Spanned, StringId};
use crate::parser2::macros::{macros, struct_decl, MacroRead, Macros};
use crate::parser2::quicklex::Token as LexToken;
use crate::parser2::token_tree::Handle;
use crate::parser2::token_tree::TokenTree;

use codespan::CodeMap;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use derive_new::new;

#[derive(Debug, Copy, Clone)]
enum NextAction {
    Top,
    Macro(StringId),
}

#[derive(Debug, Copy, Clone)]
pub struct ScopeId {
    id: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct BindingId {
    id: usize,
}

pub struct ReferenceId {
    scope_id: ScopeId,
    binding_id: BindingId,
}

#[derive(Debug)]
pub struct Scope {
    id: ScopeId,
    parent: Option<ScopeId>,
    bindings: HashMap<StringId, BindingId>,
}

impl Scope {
    fn has(&self, name: &StringId) -> bool {
        self.bindings.contains_key(&name)
    }

    fn bind(&mut self, name: &StringId, binding: BindingId) {
        self.bindings.insert(*name, binding);
    }
}

#[derive(Debug)]
pub struct Scopes {
    list: Vec<Scope>,
    next_binding: usize,
}

impl Scopes {
    fn new() -> Scopes {
        let root = Scope {
            id: ScopeId { id: 0 },
            parent: None,
            bindings: HashMap::new(),
        };

        Scopes {
            list: vec![root],
            next_binding: 0,
        }
    }

    fn next_binding(&mut self) -> BindingId {
        let next = self.next_binding;

        self.next_binding += 1;

        BindingId { id: next }
    }

    fn get(&self, id: &ScopeId) -> &Scope {
        &self.list[id.id]
    }

    fn get_mut(&mut self, id: &ScopeId) -> &mut Scope {
        &mut self.list[id.id]
    }

    fn has(&self, scope: &ScopeId, name: &StringId) -> bool {
        let scope = self.get(scope);

        scope.has(name)
    }

    fn bind(&mut self, scope: &ScopeId, name: &StringId, binding: BindingId) {
        let scope = self.get_mut(scope);

        scope.bind(name, binding);
    }

    fn root(&self) -> ScopeId {
        ScopeId { id: 0 }
    }

    fn child(&mut self, parent: &ScopeId) -> ScopeId {
        let id = ScopeId {
            id: self.list.len(),
        };
        let scope = Scope {
            id,
            parent: Some(*parent),
            bindings: HashMap::new(),
        };

        self.list.push(scope);

        id
    }
}

struct File {
    scopes: Vec<Scope>,
}

#[derive(Debug)]
struct AnnotatedToken {
    token: Token,
    scope_parent: ScopeId,
}

#[derive(Debug, Copy, Clone)]
pub enum Token {
    Binding { scope: ScopeId, name: StringId },
    Reference { scope: ScopeId, name: StringId },
    Export { scope: ScopeId, name: StringId },
    Label(StringId),
    Sigil(StringId),
    String(StringId),
    NonSemantic(NonSemantic),
    EOF,
}

impl DebugModuleTable for Token {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            Token::Binding { name, .. } => {
                write!(f, "Binding({:?})", Debuggable::from(name, table))
            }
            Token::Reference { name, .. } => {
                write!(f, "Reference({:?})", Debuggable::from(name, table))
            }
            Token::Export { name, .. } => write!(f, "Export({:?})", Debuggable::from(name, table)),
            Token::Label(name) => write!(f, "Label({:?})", Debuggable::from(name, table)),
            Token::Sigil(name) => write!(f, "#{:?}#", Debuggable::from(name, table)),
            Token::String(name) => write!(f, "\"{:?}\"", Debuggable::from(name, table)),
            Token::NonSemantic(NonSemantic::Comment(_)) => write!(f, "<comment>"),
            Token::NonSemantic(NonSemantic::Whitespace(_)) => write!(f, "<whitespace>"),
            Token::NonSemantic(NonSemantic::Newline) => write!(f, "<newline>"),
            Token::EOF => write!(f, "<EOF>"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum NonSemantic {
    Comment(StringId),
    Whitespace(StringId),
    Newline,
}

struct AnnotatedShape {
    tokens: Vec<AnnotatedToken>,
}

#[derive(Debug, new)]
pub struct LiteParser<'codemap> {
    tokens: Vec<Spanned<LexToken>>,
    macros: Macros,
    table: ModuleTable,
    codemap: &'codemap CodeMap,

    #[new(value = "Scopes::new()")]
    scopes: Scopes,

    #[new(value = "vec![]")]
    annotated: Vec<AnnotatedToken>,

    #[new(value = "0")]
    pos: usize,

    #[new(value = "vec![]")]
    out_tokens: Vec<Spanned<Token>>,

    #[new(value = "TokenTree::new()")]
    tree: TokenTree,
}

#[derive(Debug, Copy, Clone)]
pub enum RelativePosition {
    Hoist,
    After,
}

pub const ALLOW_NEWLINE: AllowPolicy = AllowPolicy(0b0001);
pub const ALLOW_EOF: AllowPolicy = AllowPolicy(0b0010);
pub const ALLOW_NONE: AllowPolicy = AllowPolicy(0b0000);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AllowPolicy(u8);

impl std::ops::BitOr for AllowPolicy {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        AllowPolicy(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for AllowPolicy {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        AllowPolicy(self.0 & rhs.0)
    }
}

impl AllowPolicy {
    fn has(&self, policy: AllowPolicy) -> bool {
        (self.0 & policy.0) != 0
    }
}

pub enum IdPolicy {
    Export { scope: ScopeId, hoist: bool },
    Bind(ScopeId),
    Refer(ScopeId),
    Label,
}

#[derive(Debug, Copy, Clone)]
pub enum Expected {
    AnyIdentifier,
    Identifier(StringId),
    Sigil(StringId),
}

impl Expected {
    fn translate(&self, lex_token: Spanned<LexToken>, id: fn(StringId) -> Token) -> Spanned<Token> {
        let token = match self {
            Expected::AnyIdentifier | Expected::Identifier(_) => id(lex_token.data()),
            Expected::Sigil(_) => Token::Sigil(lex_token.data()),
        };

        Spanned::wrap_span(token, lex_token.span())
    }

    fn matches(&self, token: &LexToken) -> bool {
        match self {
            Expected::AnyIdentifier => token.is_id(),
            Expected::Identifier(s) => token.is_id_named(*s),
            Expected::Sigil(s) => token.is_sigil_named(*s),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ExpectedId {
    AnyIdentifier,
    Identifier(StringId),
}

impl ExpectedId {
    fn matches(&self, token: &LexToken) -> bool {
        match self {
            ExpectedId::AnyIdentifier => token.is_id(),
            ExpectedId::Identifier(s) => token.is_id_named(*s),
        }
    }

    fn translate(&self, lex_token: Spanned<LexToken>, id: fn(StringId) -> Token) -> Spanned<Token> {
        let token = id(lex_token.data());

        Spanned::wrap_span(token, lex_token.span())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MaybeTerminator {
    Token(Spanned<Token>),
    Terminator(Spanned<Token>),
}

impl DebugModuleTable for MaybeTerminator {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        match self {
            MaybeTerminator::Token(token) => token.debug(f, table),
            MaybeTerminator::Terminator(token) => token.debug(f, table),
        }
    }
}

impl AllowPolicy {
    fn include_newline(&self) -> bool {
        *self == ALLOW_NEWLINE
    }
}

const EOF: Spanned<Token> = Spanned(Token::EOF, Span::EOF);

pub struct ParseResult {
    tree: TokenTree,
    tokens: Vec<Spanned<Token>>,
}

impl LiteParser<'codemap> {
    pub fn process(mut self) -> Result<ParseResult, ParseError> {
        while self.pos < self.tokens.len() {
            self.process_macro(self.root_scope())?;
        }

        println!("{:#?}", DebuggableVec::from(&self.out_tokens, self.table()));

        Ok(ParseResult {
            tree: self.tree,
            tokens: self.out_tokens,
        })
    }

    pub fn table(&self) -> &ModuleTable {
        &self.table
    }

    fn root_scope(&self) -> ScopeId {
        self.scopes.root()
    }

    fn get_macro(&mut self, id: Spanned<StringId>) -> Result<Arc<MacroRead>, ParseError> {
        self.macros.get(*id).ok_or_else(|| {
            ParseError::new(
                format!("No macro in scope {:?}", Debuggable::from(&id, &self.table)),
                id.span(),
            )
        })
    }

    fn process_macro(&mut self, scope: ScopeId) -> Result<(), ParseError> {
        match self.consume_next_id(IdPolicy::Label, ALLOW_NEWLINE | ALLOW_EOF)? {
            None => Ok(()),
            Some(id) => {
                let macro_def = self.get_macro(id)?;

                macro_def.read(scope, self)?;

                Ok(())
            }
        }
    }

    pub fn expect_id_until(
        &mut self,
        allow: AllowPolicy,
        expected: ExpectedId,
        _token: fn(StringId) -> Token,
        terminator: Expected,
    ) -> Result<MaybeTerminator, ParseError> {
        let next = self.consume_next_token(allow)?;

        match next {
            Spanned(LexToken::EOF, ..) => {
                self.push_out(EOF);
                Ok(MaybeTerminator::Token(EOF))
            }
            Spanned(id, ..) => match id {
                _ if terminator.matches(&id) => {
                    let token = terminator.translate(next, Token::Label);
                    self.push_out(token);
                    Ok(MaybeTerminator::Terminator(token))
                }
                _ if expected.matches(&id) => {
                    let token = expected.translate(next, Token::Label);
                    self.push_out(token);
                    Ok(MaybeTerminator::Token(token))
                }
                other => {
                    return Err(ParseError::new(
                        format!("Unexpected token {:?}", other),
                        next.span(),
                    ))
                }
            },
        }
    }

    pub fn sigil(&self, sigil: &str) -> Expected {
        let id = self.table.get(&sigil).expect(&format!(
            "Expected sigil {}, but none was registered",
            sigil
        ));

        Expected::Sigil(id)
    }

    pub fn expect_type(
        &mut self,
        whitespace: AllowPolicy,
        scope: ScopeId,
    ) -> Result<Handle, ParseError> {
        self.tree.start();
        self.tree.mark_type();
        self.consume_next_id(IdPolicy::Refer(scope), whitespace)?;
        let handle = self.tree.end();

        Ok(handle)
    }

    pub fn expect_sigil(&mut self, sigil: &str, allow: AllowPolicy) -> Result<(), ParseError> {
        let id = self.table.get(&sigil);

        match id {
            None => unimplemented!(),

            Some(id) => match self.consume_next_token(allow)? {
                Spanned(LexToken::EOF, ..) => Ok(()),

                Spanned(LexToken::Sigil(sigil), span) if sigil == id => {
                    let token = Token::Sigil(id);
                    self.push_out(Spanned::wrap_span(token, span));
                    Ok(())
                }

                other => Err(ParseError::new(
                    format!("Unexpected {:?}", *other),
                    other.span(),
                )),
            },
        }
    }

    pub fn export_name(
        &mut self,
        scope_id: ScopeId,
        relative: RelativePosition,
        _allow_newline: bool,
    ) -> Result<Spanned<BindingId>, ParseError> {
        let id = self.consume_next_id(
            IdPolicy::Export {
                hoist: true,
                scope: scope_id,
            },
            ALLOW_NEWLINE,
        )?;

        let id = id.expect("BUG: EOF is not allowed in export_name");

        match relative {
            RelativePosition::Hoist => {
                let scope = self.scopes.get_mut(&scope_id);

                if scope.has(&id) {
                    return Err(ParseError::new(
                        format!("Cannot create two instances of {}", self.table.lookup(&id)),
                        id.span(),
                    ));
                }
                let binding = self.scopes.next_binding();
                self.scopes.bind(&scope_id, &id, binding);
                Ok(Spanned::wrap_span(binding, id.span()))
            }
            RelativePosition::After => unimplemented!(),
        }
    }

    fn consume(&mut self) -> Spanned<LexToken> {
        assert!(
            self.pos == self.out_tokens.len(),
            "BUG: Didn't annotate all of the tokens\nnext={:?}; pos={}\nin_tokens={:?}\nout_tokens={:?}",
            Debuggable::from(&self.tokens[self.pos], self.table()),
            self.pos,
            DebuggableVec::from(&self.tokens, self.table()),
            DebuggableVec::from(&self.out_tokens, self.table())
        );

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

    fn consume_next_token(&mut self, allow: AllowPolicy) -> Result<Spanned<LexToken>, ParseError> {
        loop {
            let token = self.maybe_consume();

            println!("raw token={:?}", token);

            let token = match token {
                None if allow.has(ALLOW_EOF) => {
                    return Ok(Spanned::wrap_span(LexToken::EOF, Span::EOF))
                }
                None => return Err(ParseError::new(format!("Unexpected EOF"), Span::EOF)),
                Some(token) => token,
            };

            match *token {
                LexToken::Whitespace(string) => self.push_out(Spanned::wrap_span(
                    Token::NonSemantic(NonSemantic::Whitespace(string)),
                    token.span(),
                )),
                LexToken::Newline if allow.has(ALLOW_NEWLINE) => self.push_out(Spanned::wrap_span(
                    Token::NonSemantic(NonSemantic::Newline),
                    token.span(),
                )),
                _ => return Ok(token),
            }
        }
    }

    fn consume_next_id(
        &mut self,
        id_policy: IdPolicy,
        allow: AllowPolicy,
    ) -> Result<Option<Spanned<StringId>>, ParseError> {
        let next = self.consume_next_token(allow)?;

        let token = match *next {
            LexToken::EOF if allow.has(ALLOW_EOF) => return Ok(None),
            LexToken::EOF => {
                return Err(ParseError::new(
                    "Unexpected EOF in macro expansion, TODO".to_string(),
                    Span::EOF,
                ))
            }
            _ => {
                println!("{:?}", next.as_id());

                next.as_id()
            }
        }?;

        let name = *token;

        let out_token = match id_policy {
            IdPolicy::Label => Token::Label(name),
            IdPolicy::Bind(scope_id) => Token::Binding {
                scope: scope_id,
                name,
            },
            IdPolicy::Refer(scope_id) => Token::Reference {
                scope: scope_id,
                name,
            },
            IdPolicy::Export { scope, hoist } => {
                if hoist == false {
                    unimplemented!(
                        "Exports that only refer to later in the scope are not yet implemented"
                    );
                }

                Token::Export { scope, name }
            }
        };

        self.out_tokens
            .push(Spanned::wrap_span(out_token, token.span()));

        Ok(Some(token))
    }

    fn push_single(&mut self, _token: Spanned<Token>) {}

    fn push_out(&mut self, token: Spanned<Token>) {
        println!("Pushing token: {:?}", token);
        self.out_tokens.push(token)
    }
}

fn expect(
    token: Option<Spanned<LexToken>>,
    condition: impl FnOnce(LexToken) -> bool,
) -> Result<Spanned<LexToken>, ParseError> {
    match token {
        None => Err(ParseError::new(format!("Unexpected EOF"), Span::EOF)),

        Some(token) if condition(*token) => Ok(token),

        Some(other) => Err(ParseError::new(
            format!("Unexpected {:?}", other),
            other.span(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::LiteParser;

    use crate::parser::ast::DebuggableVec;
    use crate::parser::lexer_helpers::ParseError;
    use crate::parser::reporting::print_parse_error;
    use crate::parser::{Span, Spanned};
    use crate::parser2::macros::{macros, Macros};
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
              msg: String,
              ^^^~^~~~~~~^ @msg@ #:# ws @String@ #,#
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

        let builtin_macros = macros(ann.table());

        let parser = LiteParser::new(tokens, builtin_macros, ann.table().clone(), ann.codemap());

        match parser.process() {
            Ok(_) => {}
            Err(e) => print_parse_error(e, ann.codemap()),
        };
    }
}
