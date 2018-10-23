use crate::prelude::*;

use crate::parser2::reader::EOF;
use crate::{LexToken, PairedDelimiter, ParseError, Reader};

pub struct Paired<'reader> {
    tokens: &'reader [Spanned<LexToken>],
    pos: usize,
    delimiters: Vec<Spanned<PairedDelimiter>>,
}

impl Paired<'reader> {
    crate fn start(reader: &Reader, del: Spanned<PairedDelimiter>) -> Paired<'reader> {
        let (tokens, pos) = reader.tokens();
        let delimiters = vec![del];

        Paired {
            tokens,
            pos,
            delimiters,
        }
    }

    pub fn process(&mut self) -> Result<usize, ParseError> {
        loop {
            let next = self.consume();

            match next.node() {
                LexToken::Whitespace(..) => continue,
                LexToken::Identifier(..) => continue,
                LexToken::Comment(..) => continue,
                LexToken::String(..) => continue,
                LexToken::Newline => continue,
                LexToken::EOF => return Err(ParseError::new(format!("Unexpected EOF"), EOF)),
            }
        }
    }

    fn consume(&mut self) -> Spanned<LexToken> {
        let token = self.tokens[self.pos];

        self.pos += 1;

        token
    }

    fn current_delimiter(&self) -> Spanned<PairedDelimiter> {
        self.delimiters[self.delimiters.len() - 1]
    }

    fn push_delimiter(&mut self, del: Spanned<PairedDelimiter>) {
        self.delimiters.push(del);
    }

    fn pop_delimiter(&mut self, span: Span) -> Result<Spanned<PairedDelimiter>, ParseError> {
        match self.delimiters.pop() {
            Some(del) => Ok(del),
            None => Err(ParseError::new(format!("Unbalanced delimiter"), span)),
        }
    }
}
