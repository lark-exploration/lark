use super::*;

impl Def {
    pub fn parameter(mut self, field: Field) -> Self {
        self.parameters.push(field);
        self
    }

    pub fn ret(mut self, ty: Option<Spanned<Type>>) -> Self {
        self.ret = ty;
        self
    }
}
