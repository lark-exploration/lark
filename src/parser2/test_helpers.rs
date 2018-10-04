use codespan::ByteIndex;
use crate::parser::lexer_helpers::ParseError;
use crate::parser::test_helpers::{LineTokenizer, Token};
use crate::parser::{ast, ModuleTable, Span, Spanned, StringId};

use codespan::{ByteOffset, CodeMap};
use derive_new::new;
use itertools::Itertools;
use log::{debug, trace};
use std::collections::HashMap;
use unicode_xid::UnicodeXID;

pub fn process(source: &str) -> (String, Annotations) {
    let codemap = CodeMap::new();
    extract(&source, codemap, 1)
}

#[derive(Debug)]
pub enum Annotation {
    Whitespace(Span),
    Newline(Span),
    Identifier(Span),
    Sigil(Span),
}

fn extract(s: &str, codemap: CodeMap, mut codespan_start: u32) -> (String, Annotations) {
    let mut source = String::new();
    let mut t2 = ModuleTable::new();
    let mut anns = vec![];
    let mut spans = vec![];

    for (i, mut chunk) in s.lines().chunks(2).into_iter().enumerate() {
        let line = chunk.next().expect("line in chunk");
        let annotations = chunk.next().expect("annotation in chunk");

        source.push_str(&line);
        source.push('\n');

        debug!("line:        {} {:?}", i, line);
        debug!("annotations: {} {:?}", i, annotations);

        let tokens = LineTokenizer::new(&mut t2, annotations, 0);
        let tokens: Result<Vec<(ByteIndex, Token, ByteIndex)>, ParseError> = tokens.collect();

        for token in tokens.unwrap() {
            trace!(target: "lark::parser::test::extract", "token={:?} start={:?}", token, codespan_start);

            match token {
                (start, token, end) => match token {
                    Token::Underline => {
                        trace!(target: "lark::parser::test::extract",
                            "^^^ start={:?} end={:?} codespan_start={:?}",
                            start,
                            end,
                            codespan_start
                        );

                        spans.push(Span::from(
                            start + ByteOffset(codespan_start as i64),
                            end + ByteOffset(codespan_start as i64),
                        ))
                    }
                    Token::Name(id) => {
                        let (name, snip, span) = ident(id, &anns, &spans, &t2, &source);

                        assert_eq!(&name[1..name.len() - 1], snip, "annotation matches source");

                        assert!(
                            UnicodeXID::is_xid_start(snip.chars().next().unwrap()),
                            "source id starts with an id char; source={:?}",
                            snip
                        );
                        assert!(
                            snip[1..].chars().all(|i| UnicodeXID::is_xid_continue(i)),
                            "source id contains only id chars; source={:?}",
                            snip
                        );

                        anns.push(Annotation::Identifier(span));
                    }

                    Token::WsKeyword => {
                        let (snip, span) = sigil(&anns, &spans, &source);

                        assert!(
                            snip.chars().all(|i| i.is_whitespace()),
                            "annotation ws matches source"
                        );

                        anns.push(Annotation::Whitespace(span))
                    }

                    Token::Sigil(id) => {
                        let (name, snip, span) = ident(id, &anns, &spans, &t2, &source);

                        assert_eq!(&name[1..name.len() - 1], snip, "annotation matches source");

                        anns.push(Annotation::Sigil(span));
                    }

                    Token::Whitespace => {}
                },
            }
        }

        // lines.insert(name.to_string(), i as u32);
        // span_map.insert(i as u32, spans);

        codespan_start = (source.len() as u32) + 1;
    }

    (source, Annotations::new(codemap, t2, anns))
}

fn ident(
    id: StringId,
    anns: &[Annotation],
    spans: &[Span],
    table: &'table ModuleTable,
    source: &'source str,
) -> (&'table str, &'source str, Span) {
    let pos = anns.len();
    let span = spans[pos];

    let name = table.lookup(id);
    let snip = &source[span.to_range(-1)];

    trace!(target: "lark::parser::test::extract",
        "name={:?} snip={:?} span={:?} source={:?}",
        name, snip, span, source
    );

    (name, snip, span)
}

fn sigil(anns: &[Annotation], spans: &[Span], source: &'source str) -> (&'source str, Span) {
    let pos = anns.len();
    let span = spans[pos];

    let snip = &source[span.to_range(-1)];

    trace!(target: "lark::parser::test::extract",
        "snip={:?} span={:?} source={:?}",
        snip, span, source
    );

    (snip, span)
}

#[derive(Debug, new)]
pub struct Annotations {
    codemap: CodeMap,
    table: ModuleTable,
    tokens: Vec<Annotation>,
}

pub trait Position: Copy {
    fn pos(&self) -> (&str, u32);
}

impl Position for (&str, u32) {
    fn pos(&self) -> (&str, u32) {
        (self.0, self.1)
    }
}

impl Annotations {
    pub fn codemap(&mut self) -> &mut CodeMap {
        &mut self.codemap
    }

    pub fn table(&mut self) -> &mut ModuleTable {
        &mut self.table
    }

    pub fn tokens(&mut self) -> &mut [Annotation] {
        &mut self.tokens
    }
}
