use codespan::ByteIndex;
use codespan::ByteSpan;
use crate::parser::ast;
use crate::parser::keywords::{KEYWORDS, SIGILS};
use crate::parser::pos::Span;
use crate::parser::pos::Spanned;
use crate::parser::program::Program;
use crate::parser::program::StringId;
use crate::parser::token::Token;

pub struct ModuleBuilder<'program> {
    program: &'program mut Program,
    module: ast::Module,
    pos: u32,
}

impl ModuleBuilder<'program> {
    pub fn new(program: &mut Program) -> ModuleBuilder<'_> {
        ModuleBuilder {
            program,
            module: ast::Module::new(vec![]),
            pos: 0,
        }
    }

    pub fn build_struct(mut self, name: &str, f: impl FnOnce(StructBuilder<'_>)) -> Self {
        let start = self.pos;
        let keyword = self.keyword("struct");
        self.ws();
        let name = self.add(name);
        self.ws();
        self.sigil("{");
        self.lf();

        let s = {
            let module = &mut self;
            let struct_builder = StructBuilder {
                module,
                start,
                s: ast::Struct::new(name, vec![], Span::Synthetic),
            };

            f(struct_builder);
            struct_builder.finish()
        };

        self.module = self.module.add_struct(s);

        self
    }

    pub fn finish(self) -> ast::Module {
        self.module
    }

    pub fn ws(&mut self) {
        self.pos += 1;
    }

    pub fn lf(&mut self) {
        self.pos += 1;
    }

    fn keyword(&mut self, s: &str) -> Spanned<Token> {
        let (token, len) = KEYWORDS.match_keyword(s).unwrap();

        let name = Spanned::from(token, ByteIndex(self.pos), ByteIndex(self.pos + len));

        self.pos += len;

        name
    }

    fn sigil(&mut self, s: &str) -> Spanned<Token> {
        let (token, len) = SIGILS.match_keyword(s).unwrap();

        let name = Spanned::from(token, ByteIndex(self.pos), ByteIndex(self.pos + len));

        self.pos += len;

        name
    }

    fn add(&mut self, s: &str) -> Spanned<StringId> {
        let id = self.program.intern(s);
        let ret = Spanned::from(
            id,
            ByteIndex(self.pos),
            ByteIndex(self.pos + s.len() as u32),
        );

        self.pos += s.len() as u32;

        ret
    }
}

pub struct StructBuilder<'a> {
    module: &'a mut ModuleBuilder<'a>,
    start: u32,
    s: ast::Struct,
}

impl StructBuilder<'a> {
    pub fn finish(self) -> ast::Struct {
        self.module.lf();
        self.module.sigil("}");

        self.s.spanned(self.start, self.module.pos)
    }
}
