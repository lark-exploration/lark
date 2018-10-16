use super::*;

impl Struct {
    crate fn build(name: &'input str, table: &mut ModuleTable) -> Struct {
        Struct {
            name: Spanned::synthetic(table.intern(&name)),
            fields: vec![],
            span: Span::Synthetic,
        }
    }

    crate fn spanned(mut self, start: u32, end: u32) -> Struct {
        self.span = Span::from(ByteIndex(start), ByteIndex(end));
        self
    }

    crate fn name_spanned(mut self, start: u32, end: u32) -> Struct {
        self.name.1 = Span::from(ByteIndex(start), ByteIndex(end));
        self
    }

    crate fn field(mut self, field: Field) -> Struct {
        self.fields.push(field);
        self
    }
}

impl Def {
    pub fn parameter(mut self, field: Field) -> Self {
        self.parameters.push(field);
        self
    }

    pub fn ret(mut self, ty: Option<Spanned<Type>>) -> Self {
        self.ret = ty;
        self
    }

    crate fn spanned(mut self, start: u32, end: u32) -> Self {
        self.span = Span::from(ByteIndex(start), ByteIndex(end));
        self
    }
}

impl Module {
    crate fn build() -> Module {
        Module { items: vec![] }
    }

    crate fn add_struct(mut self, s: Struct) -> Module {
        self.items.push(Arc::new(Item::Struct(s)));
        self
    }

    crate fn def(mut self, def: Def) -> Module {
        self.items.push(Arc::new(Item::Def(def)));
        self
    }
}
