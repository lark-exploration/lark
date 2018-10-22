use crate::parser::ParseError;
use crate::parser2::reader::Reader;
use crate::parser2::{Handle, LiteParser, ScopeId};

use derive_new::new;

#[derive(Debug, new)]
pub struct ExprParser;

impl ExprParser {
    pub fn extent(&mut self, reader: &mut Reader<'_>) -> Result<Handle, ParseError> {
        reader.tree().start();
        reader.tree().mark_expr();

        let handle = reader.tree().end();

        Ok(handle)
    }
}
