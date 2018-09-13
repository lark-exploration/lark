use codespan::{ByteIndex, ByteSpan};
use crate::parser::ast::{self, Field, Mode, Type};
use crate::parser::keywords::{KEYWORDS, SIGILS};
use crate::parser::pos::{HasSpan, Span, Spanned};
use crate::parser::program::ModuleTable;
use crate::parser::program::StringId;
use crate::parser::token::Token;

pub struct ModuleBuilder<'program> {
    program: &'program mut ModuleTable,
    module: ast::Module,
    pos: u32,
}

impl ModuleBuilder<'program> {
    pub fn new(program: &mut ModuleTable, start: u32) -> ModuleBuilder<'_> {
        ModuleBuilder {
            program,
            module: ast::Module::new(vec![]),
            pos: start,
        }
    }

    pub fn add_struct(
        mut self,
        name: &str,
        f: impl FnOnce(StructBuilder<'_>) -> StructBuilder<'_>,
    ) -> Self {
        let struct_start = self.pos;
        let keyword = self.keyword("struct");
        self.ws();
        let name = self.add(name);
        self.ws();
        self.sigil("{");
        self.lf();

        let start_pos = self.pos;

        let (s, mut builder) = {
            let struct_builder = StructBuilder {
                module: self,
                struct_start,
                start_pos,
                s: ast::Struct::new(name, vec![], Span::Synthetic),
            };

            let struct_builder = f(struct_builder);
            struct_builder.finish()
        };

        builder.module = builder.module.add_struct(s);
        builder.lf();
        builder.lf();

        builder
    }

    pub fn def(mut self, name: &str, f: impl FnOnce(DefBuilder<'_>) -> DefBuilder<'_>) -> Self {
        let def_start = self.pos;
        let keyword = self.keyword("def");
        self.ws();
        let name = self.add(name);

        let start_pos = self.pos;

        let (def, mut builder) = {
            self.sigil("(");

            let def_builder = DefBuilder {
                module: self,
                def_start,
                start_pos,
                def: ast::Def::new(name, vec![], None, ast::Block::new(vec![]), Span::Synthetic),
                first: true,
            };

            let def_builder = f(def_builder);
            def_builder.finish()
        };

        builder.module = builder.module.def(def);
        builder.lf();
        builder.lf();

        builder
    }

    pub fn finish(self) -> ast::Module {
        self.module
    }

    pub fn ws(&mut self) {
        self.pos += 1;
    }

    pub fn indent(&mut self) {
        self.pos += 2;
    }

    pub fn lf(&mut self) {
        self.pos += 1;
    }

    fn mode(&mut self, mode: Option<Mode>) -> Option<Spanned<Mode>> {
        match mode {
            None => None,
            Some(Mode::Owned) => {
                let keyword = self.keyword("own");
                self.ws();

                Some(Spanned::wrap_span(Mode::Owned, keyword.span))
            }

            Some(Mode::Shared) => None,

            Some(Mode::Borrowed) => {
                let keyword = self.keyword("borrowed");
                self.ws();

                Some(Spanned::wrap_span(Mode::Borrowed, keyword.span))
            }
        }
    }

    fn keyword(&mut self, s: &str) -> Spanned<Token> {
        let (token, len) = KEYWORDS.match_token(s).unwrap();

        let name = Spanned::from(token, ByteIndex(self.pos), ByteIndex(self.pos + len));

        self.pos += len;

        name
    }

    fn sigil(&mut self, s: &str) -> Spanned<Token> {
        let (token, len) = SIGILS.match_token(s).unwrap();

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
    module: ModuleBuilder<'a>,
    struct_start: u32,
    start_pos: u32,
    s: ast::Struct,
}

impl StructBuilder<'a> {
    pub fn field(mut self, name: &str, mode: Option<Mode>, id: &str) -> Self {
        self.module.indent();
        let name = self.module.add(name);
        self.module.sigil(":");
        self.module.ws();

        let mode: Option<Spanned<Mode>> = match mode {
            None => None,
            Some(Mode::Owned) => {
                let keyword = self.module.keyword("own");
                self.module.ws();

                Some(Spanned::wrap_span(Mode::Owned, keyword.span))
            }

            Some(Mode::Shared) => None,

            Some(Mode::Borrowed) => {
                let keyword = self.module.keyword("borrowed");
                self.module.ws();

                Some(Spanned::wrap_span(Mode::Borrowed, keyword.span))
            }
        };

        let type_name = self.module.add(id);
        let ty_span = match mode {
            None => type_name.span,
            Some(mode) => mode.span.to(type_name.span),
        };

        let ty = Spanned::wrap_span(Type::new(mode, type_name), ty_span);
        let field_span = name.span().to(ty.span());

        self.module.sigil(",");
        self.module.lf();

        self.s = self.s.field(Field::new(name, ty, field_span));

        self
    }

    pub fn finish(mut self) -> (ast::Struct, ModuleBuilder<'a>) {
        self.module.sigil("}");

        (
            self.s.spanned(self.struct_start, self.module.pos),
            self.module,
        )
    }
}

pub struct DefBuilder<'a> {
    module: ModuleBuilder<'a>,
    def_start: u32,
    start_pos: u32,
    def: ast::Def,
    first: bool,
}

impl DefBuilder<'a> {
    pub fn param(mut self, name: &str, mode: Option<Mode>, id: &str) -> Self {
        if self.first {
            self.first = false;
        } else {
            self.module.sigil(",");
            self.module.ws();
        }

        let name = self.module.add(name);
        self.module.sigil(":");
        self.module.ws();

        let mode: Option<Spanned<Mode>> = match mode {
            None => None,
            Some(Mode::Owned) => {
                let keyword = self.module.keyword("own");
                self.module.ws();

                Some(Spanned::wrap_span(Mode::Owned, keyword.span))
            }

            Some(Mode::Shared) => None,

            Some(Mode::Borrowed) => {
                let keyword = self.module.keyword("borrowed");
                self.module.ws();

                Some(Spanned::wrap_span(Mode::Borrowed, keyword.span))
            }
        };

        let type_name = self.module.add(id);
        let ty_span = match mode {
            None => type_name.span,
            Some(mode) => mode.span.to(type_name.span),
        };

        let ty = Spanned::wrap_span(Type::new(mode, type_name), ty_span);
        let param_span = name.span().to(ty.span());
        let param_span = name.span().to(ty.span());

        self.def = self.def.parameter(Field::new(name, ty, param_span));
        self
    }

    pub fn returns_void() -> Self {
        unimplemented!()
    }

    pub fn returns(mut self, mode: Option<Mode>, ty: &str) -> Self {
        self.module.sigil(")");
        self.module.ws();
        self.module.sigil("->");
        self.module.ws();

        let mode = self.module.mode(mode);

        let ty = match mode {
            None => {
                let name = self.module.add(ty);
                Spanned::wrap_span(Type::new(None, name), name.span())
            }

            Some(mode) => {
                self.module.ws();
                let name = self.module.add(ty);
                Spanned::wrap_span(Type::new(Some(mode), name), mode.span().to(name.span()))
            }
        };

        self.def = self.def.ret(Some(ty));
        self.module.ws();
        self.module.sigil("{");
        self.module.lf();

        self
    }

    pub fn finish(mut self) -> (ast::Def, ModuleBuilder<'a>) {
        self.module.sigil("}");

        (
            self.def.spanned(self.def_start, self.module.pos),
            self.module,
        )
    }
}
