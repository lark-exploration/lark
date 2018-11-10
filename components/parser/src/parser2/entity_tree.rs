use crate::prelude::*;

use crate::intern::ModuleTable;
use crate::parser2::macros::Term;
use crate::parser2::token_tree::{TokenPos, TokenSpan};
use crate::LexToken;

use derive_new::new;
use map::FxIndexMap;
use std::cell::Cell;
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EntityKind {
    Struct,
    Def,
    File,
}

#[derive(Debug, Eq, PartialEq)]
pub struct EntityTree {
    entity: Entity,
    children: FxIndexMap<GlobalIdentifier, EntityTree>,
}

impl EntityTree {
    pub fn debug(
        &self,
        table: &ModuleTable,
        tokens: &[Spanned<LexToken>],
        entities: &'terms Entities,
    ) -> DebugEntityTree<'terms> {
        DebugEntityTree {
            entity: self.entity.debug(table, tokens, entities),
            children: self
                .children
                .iter()
                .map(|(k, v)| {
                    (
                        table.lookup(k).to_string(),
                        v.debug(table, tokens, entities),
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct DebugEntityTree<'terms> {
    entity: DebugEntity<'terms>,
    children: FxIndexMap<String, DebugEntityTree<'terms>>,
}

#[derive(Debug, Default, new)]
pub struct EntityTreeBuilder {
    #[new(value = "None")]
    parent: Option<Box<EntityTreeBuilder>>,

    #[new(value = "None")]
    entity: Option<Entity>,

    #[new(value = "FxIndexMap::default()")]
    children: FxIndexMap<GlobalIdentifier, EntityTree>,

    #[new(value = "Some(vec![])")]
    terms: Option<Vec<Box<dyn Term>>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Entity {
    span: TokenSpan,
    name: GlobalIdentifier,
    kind: EntityKind,
    term: TermId,
}

impl Entity {
    pub fn debug(
        &self,
        table: &ModuleTable,
        tokens: &[Spanned<LexToken>],
        entities: &'terms Entities,
    ) -> DebugEntity<'terms> {
        let start = tokens[(self.span.0).0].node();
        let end = tokens[(self.span.1).0].node();
        DebugEntity {
            span: self.span,
            start: format!("{:?}", Debuggable::from(start, table)),
            end: format!("{:?}", Debuggable::from(end, table)),
            name: table.lookup(&self.name).to_string(),
            kind: self.kind,
            term: entities.get_term(&self.term),
        }
    }

    pub fn start(&self) -> TokenPos {
        self.span.0
    }

    pub fn end(&self) -> TokenPos {
        self.span.1
    }
}

#[derive(Debug)]
pub struct DebugEntity<'terms> {
    pub span: TokenSpan,
    pub start: String,
    pub end: String,
    pub name: String,
    pub kind: EntityKind,
    pub term: &'terms dyn Term,
}

impl EntityTreeBuilder {
    pub fn push(
        mut self,
        name: &GlobalIdentifier,
        start: TokenPos,
        kind: EntityKind,
    ) -> EntityTreeBuilder {
        let terms = self.terms.take();

        EntityTreeBuilder {
            parent: Some(box self),
            entity: Some(Entity {
                span: TokenSpan(start, TokenPos(0)),
                name: *name,
                kind,
                term: TermId(-1),
            }),
            children: FxIndexMap::default(),
            terms,
        }
    }

    pub fn finish(self, finish: TokenPos, term: Box<dyn Term>) -> EntityTreeBuilder {
        let EntityTreeBuilder {
            parent,
            entity,
            children,
            terms,
        } = self;

        let mut terms = terms.expect("Expected terms when finishing node");
        let term_id = TermId(terms.len() as isize);
        terms.push(term);

        let mut entity = entity.expect("Can't finish the root node");
        let id = entity.name;
        entity.span.1 = finish;
        entity.term = term_id;

        let finished = EntityTree { entity, children };

        let mut parent = parent.expect("Can't finish the root node");

        parent.children.insert(id, finished);
        parent.terms = Some(terms);

        *parent
    }

    pub fn finalize(self) -> (FxIndexMap<GlobalIdentifier, EntityTree>, Vec<Box<dyn Term>>) {
        assert!(self.parent.is_none(), "Can only finalize the root node");

        (
            self.children,
            self.terms
                .expect("Expected entity tree to have terms when finalized"),
        )
    }
}

#[derive(new)]
pub struct EntitiesBuilder {
    #[new(value = "Cell::new(EntityTreeBuilder::new())")]
    tree: Cell<EntityTreeBuilder>,
}

impl fmt::Debug for EntitiesBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = self.tree.take();
        write!(f, "{:?}", value)?;
        self.tree.set(value);

        Ok(())
    }
}

impl EntitiesBuilder {
    pub fn push(&mut self, name: &GlobalIdentifier, start: TokenPos, kind: EntityKind) {
        self.update(|tree| tree.push(name, start, kind));
    }

    pub fn finish(&mut self, finish: TokenPos, term: Box<dyn Term>) {
        self.update(|tree| tree.finish(finish, term));
    }

    pub fn finalize(self) -> Entities {
        let (tree, terms) = self.tree.into_inner().finalize();

        Entities { tree, terms }
    }

    fn update(&mut self, f: impl FnOnce(EntityTreeBuilder) -> EntityTreeBuilder) {
        let tree = f(self.tree.take());
        self.tree.set(tree);
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TermId(isize);

#[derive(Debug)]
pub struct Entities {
    tree: FxIndexMap<GlobalIdentifier, EntityTree>,
    terms: Vec<Box<dyn Term>>,
}

impl Entities {
    pub fn debug(
        &self,
        table: &ModuleTable,
        tokens: &[Spanned<LexToken>],
    ) -> FxIndexMap<String, DebugEntityTree> {
        debug_tree(&self.tree, table, tokens, self)
    }

    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn str_keys(&self, table: &'table ModuleTable) -> Vec<Text> {
        self.tree.keys().map(|id| table.lookup(id)).collect()
    }

    pub fn get_term(&self, term: &TermId) -> &dyn Term {
        assert!(term.0 != -1, "BUG: TermId of -1 is a sentinel value");

        &*self.terms[term.0 as usize]
    }

    pub fn get_entity(&self, name: &GlobalIdentifier) -> Option<&Entity> {
        self.tree.get(name).map(|i| &i.entity)
    }

    pub fn get_entity_by(&self, table: &ModuleTable, name: &str) -> Entity {
        let id = table.intern(name);
        *self
            .get_entity(&id)
            .expect(&format!("Entity {} didn't exist", name))
    }
}

fn debug_tree(
    tree: &FxIndexMap<GlobalIdentifier, EntityTree>,
    table: &ModuleTable,
    tokens: &[Spanned<LexToken>],
    entities: &'terms Entities,
) -> FxIndexMap<String, DebugEntityTree<'terms>> {
    tree.iter()
        .map(|(k, v)| {
            (
                table.lookup(k).to_string(),
                v.debug(table, tokens, entities),
            )
        })
        .collect()
}
