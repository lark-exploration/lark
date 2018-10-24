use crate::prelude::*;

use crate::parser2::reader::EOF;
use crate::{LexToken, PairedDelimiter, ParseError, Reader, ModuleTable};
use crate::parser2::token::{self, ClassifiedSigil};

pub struct Paired<'reader> {
    tokens: &'reader [Spanned<LexToken>],
    pos: usize,
    delimiters: Vec<Spanned<PairedDelimiter>>,
    table: &'reader ModuleTable
}

impl Paired<'reader> {
    crate fn start(reader: &Reader, del: Spanned<PairedDelimiter>) -> Paired<'reader> {
        let (tokens, pos) = reader.tokens();
        let delimiters = vec![del];

        Paired {
            tokens,
            pos,
            delimiters,
            table: reader.table()
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
                LexToken::Sigil(sigil) => {
                    self.process_sigil(*sigil, next.span())?;

                    if self.delimiters.len() == 0 {
                        return Ok(self.pos)
                    }
                }
                LexToken::Newline => continue,
                LexToken::EOF => return Err(ParseError::new(format!("Unexpected EOF"), Span::EOF)),
            }
        }
    }

    fn process_sigil(&mut self, sigil: token::Sigil, span: Span) -> Result<(), ParseError> {
        match sigil.classify(self.table) {
            ClassifiedSigil::OpenCurly => self.push_delimiter(Spanned::wrap_span(PairedDelimiter::Curly, span)),
            ClassifiedSigil::OpenSquare => self.push_delimiter(Spanned::wrap_span(PairedDelimiter::Square, span)),
            ClassifiedSigil::OpenRound => self.push_delimiter(Spanned::wrap_span(PairedDelimiter::Round, span)),
            sigil if self.closes_current(sigil) => { self.pop_delimiter(span)?; },
            _ => {}
        }

        Ok(())
    }

    fn consume(&mut self) -> Spanned<LexToken> {
        let token = self.tokens[self.pos];

        self.pos += 1;

        token
    }

    fn closes_current(&self, delimiter: ClassifiedSigil) -> bool {
        let current = self.current_delimiter();

        match (current.node(), delimiter) {
            (PairedDelimiter::Curly, ClassifiedSigil::CloseCurly) => true,
            (PairedDelimiter::Round, ClassifiedSigil::CloseRound) => true,
            (PairedDelimiter::Square, ClassifiedSigil::CloseSquare) => true,
            _ => false
        }
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
