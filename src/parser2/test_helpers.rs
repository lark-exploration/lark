use crate::parser::test_helpers::{LineTokenizer, Token};
use crate::parser::{ast, ModuleTable, Span, Spanned, StringId};

use codespan::{ByteOffset, CodeMap};
use derive_new::new;
use itertools::Itertools;
use log::{debug, trace};
use std::collections::HashMap;

fn extract(s: &str, codemap: CodeMap, mut codespan_start: u32) -> (String, Annotations) {
    let mut span_map = HashMap::new();
    let mut lines = HashMap::new();

    let mut source = String::new();
    let mut t2 = ModuleTable::new();

    for (i, mut chunk) in s.lines().chunks(2).into_iter().enumerate() {
        let line = chunk.next().expect("line in chunk");
        let annotations = chunk.next().expect("annotation in chunk");

        let mut spans = vec![];

        source.push_str(&line);
        source.push('\n');

        debug!("line:        {} {:?}", i, line);
        debug!("annotations: {} {:?}", i, annotations);

        let tokens = LineTokenizer::new(&mut t2, annotations, 0);
        let mut name = None;

        for token in tokens {
            trace!("{:?}", token);
            match token {
                Err(err) => panic!(err),
                Ok((start, token, end)) => match token {
                    Token::Underline => spans.push(Span::from(
                        start + ByteOffset(codespan_start as i64),
                        end + ByteOffset(codespan_start as i64),
                    )),
                    Token::Name(id) => {
                        name = Some(id);
                        break;
                    }
                    Token::Whitespace => {}
                },
            }
        }

        let name = t2.lookup(name.expect("Annotation line must have a name"));
        lines.insert(name.to_string(), i as u32);
        span_map.insert(i as u32, spans);

        codespan_start += (line.len() as u32) + 1;
    }

    (source, Annotations::new(codemap, t2, span_map, lines))
}

#[derive(Debug, new)]
struct Annotations {
    codemap: CodeMap,
    table: ModuleTable,
    spans: HashMap<u32, Vec<Span>>,
    lines: HashMap<String, u32>,
}

trait Position: Copy {
    fn pos(&self) -> (&str, u32);
}

impl Position for (&str, u32) {
    fn pos(&self) -> (&str, u32) {
        (self.0, self.1)
    }
}

impl Annotations {
    fn get(&self, pos: impl Position) -> Span {
        let (name, pos) = pos.pos();

        let line = self.lines.get(name).expect(&format!(
            "Wrong line name {}, names={:?}",
            name,
            self.lines.keys()
        ));

        let spans = self.spans.get(line).expect(&format!(
            "Wrong line number {}, len={}",
            line,
            self.spans.len()
        ));

        spans[pos as usize]
    }

    fn codemap(&mut self) -> &mut CodeMap {
        &mut self.codemap
    }

    fn table(&mut self) -> &mut ModuleTable {
        &mut self.table
    }

    fn wrap<T>(&self, value: T, left: impl Position, right: impl Position) -> Spanned<T> {
        let span = self.span(left, right);

        Spanned::wrap_span(value, span)
    }

    fn wrap_one<T>(&self, value: T, pos: impl Position) -> Spanned<T> {
        Spanned::wrap_span(value, self.get(pos))
    }

    fn mode(&self, pos: impl Position) -> Spanned<ast::Mode> {
        let src = self.src(pos);
        let mode = src.into();

        self.wrap_one(mode, pos)
    }

    fn op(&self, pos: impl Position) -> Spanned<ast::Op> {
        let src = self.src(pos);

        match src {
            "+" => self.wrap_one(ast::Op::Add, pos),
            other => panic!("Unexpected operator {:?}", other),
        }
    }

    fn pat_ident(&self, pos: impl Position) -> Spanned<ast::Pattern> {
        let id = self.ident(pos);
        self.wrap_one(ast::Pattern::Identifier(id, None), pos)
    }

    fn ty(&self, line: &str, start: u32) -> Spanned<ast::Type> {
        self.wrap_one(
            ast::Type::new(None, self.ident((line, start))),
            (line, start),
        )
    }

    fn ty_mode(&self, line: &str, start: u32) -> Spanned<ast::Type> {
        self.wrap(
            ast::Type::new(
                Some(self.mode((line, start))),
                self.ident((line, start + 1)),
            ),
            (line, start),
            (line, start + 1),
        )
    }

    fn field(&self, line: &str, start: u32) -> ast::Field {
        ast::Field::new(
            self.ident((line, start)),
            self.ty(line, start + 1),
            self.span((line, start), (line, start + 1)),
        )
    }

    fn field_mode(&self, line: &str, start: u32) -> ast::Field {
        ast::Field::new(
            self.ident((line, start)),
            self.ty_mode(line, start + 1),
            self.span((line, start), (line, start + 2)),
        )
    }

    fn shorthand(&self, pos: impl Position) -> ast::ConstructField {
        let id = self.ident(pos);

        ast::ConstructField::Shorthand(id)
    }

    fn string(&self, pos: impl Position) -> ast::Expression {
        let string = self.src(pos);
        let id = self.table.get(string).expect(&format!(
            "Missing expected string {:?}, had {:?}",
            string,
            self.table.values()
        ));

        ast::Expression::Literal(ast::Literal::String(self.wrap_one(id, pos)))
    }

    fn refers(&self, pos: impl Position) -> ast::Expression {
        let id = self.ident(pos);
        ast::Expression::Ref(id)
    }

    fn ident(&self, pos: impl Position) -> Spanned<StringId> {
        let span = self.get(pos);

        let file = self
            .codemap
            .find_file(span.to_codespan().start())
            .expect("Missing file");

        let src = file
            .src_slice(span.to_codespan())
            .expect("Missing src_slice");

        let id = self
            .table
            .get(src)
            .expect(&format!("Missing intern for {:?}", src));

        Spanned::wrap_span(id, span)
    }

    fn src(&self, pos: impl Position) -> &str {
        let span = self.get(pos);
        let file = self
            .codemap
            .find_file(span.to_codespan().start())
            .expect("Missing file");

        let src = file
            .src_slice(span.to_codespan())
            .expect("Missing src_slice");

        src
    }

    fn span(&self, from: impl Position, to: impl Position) -> Span {
        let left = self.get(from);
        let right = self.get(to);

        left.to(right)
    }
}
