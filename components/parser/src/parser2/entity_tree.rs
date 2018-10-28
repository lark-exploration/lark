use crate::prelude::*;

use crate::parser2::token_tree::{TokenPos, TokenSpan};
use crate::{LexToken, ModuleTable, StringId};

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
    children: FxIndexMap<StringId, EntityTree>,
}

impl EntityTree {
    pub fn debug(&self, table: &ModuleTable, tokens: &[Spanned<LexToken>]) -> DebugEntityTree {
        DebugEntityTree {
            entity: self.entity.debug(table, tokens),
            children: self
                .children
                .iter()
                .map(|(k, v)| (table.lookup(k).to_string(), v.debug(table, tokens)))
                .collect(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct DebugEntityTree {
    entity: DebugEntity,
    children: FxIndexMap<String, DebugEntityTree>,
}

#[derive(Debug, Eq, PartialEq, Default, new)]
pub struct EntityTreeBuilder {
    #[new(value = "None")]
    parent: Option<Box<EntityTreeBuilder>>,

    #[new(value = "None")]
    entity: Option<Entity>,

    #[new(value = "FxIndexMap::default()")]
    children: FxIndexMap<StringId, EntityTree>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Entity {
    span: TokenSpan,
    name: StringId,
    kind: EntityKind,
}

impl Entity {
    pub fn debug(&self, table: &ModuleTable, tokens: &[Spanned<LexToken>]) -> DebugEntity {
        let start = tokens[(self.span.0).0].node();
        let end = tokens[(self.span.1).0].node();
        DebugEntity {
            span: self.span,
            start: format!("{:?}", Debuggable::from(start, table)),
            end: format!("{:?}", Debuggable::from(end, table)),
            name: table.lookup(&self.name).to_string(),
            kind: self.kind,
        }
    }

    pub fn start(&self) -> TokenPos {
        self.span.0
    }

    pub fn end(&self) -> TokenPos {
        self.span.1
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct DebugEntity {
    pub span: TokenSpan,
    pub start: String,
    pub end: String,
    pub name: String,
    pub kind: EntityKind,
}

impl EntityTreeBuilder {
    pub fn push(self, name: &StringId, start: TokenPos, kind: EntityKind) -> EntityTreeBuilder {
        EntityTreeBuilder {
            parent: Some(box self),
            entity: Some(Entity {
                span: TokenSpan(start, TokenPos(0)),
                name: *name,
                kind,
            }),
            children: FxIndexMap::default(),
        }
    }

    pub fn finish(self, finish: TokenPos) -> EntityTreeBuilder {
        let EntityTreeBuilder {
            parent,
            entity,
            children,
        } = self;

        let mut entity = entity.expect("Can't finish the root node");
        let id = entity.name;
        entity.span.1 = finish;

        let finished = EntityTree { entity, children };

        let mut parent = parent.expect("Can't finish the root node");

        parent.children.insert(id, finished);

        *parent
    }

    pub fn finalize(self) -> FxIndexMap<StringId, EntityTree> {
        assert!(self.parent == None, "Can only finalize the root node");

        self.children
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
    pub fn push(&mut self, name: &StringId, start: TokenPos, kind: EntityKind) {
        self.update(|tree| tree.push(name, start, kind));
        // let tree = self.tree.take().push(name, start, kind);
        // self.tree.set(tree);
        // self.tree.update(|tree| tree.push(name, start, kind));
    }

    pub fn finish(&mut self, finish: TokenPos) {
        self.update(|tree| tree.finish(finish));
        // let tree = self.tree.take().finish(finish);
        // self.tree.set(tree);
    }

    pub fn finalize(self) -> Entities {
        Entities {
            tree: self.tree.into_inner().finalize(),
        }
    }

    fn update(&mut self, f: impl FnOnce(EntityTreeBuilder) -> EntityTreeBuilder) {
        let tree = f(self.tree.take());
        self.tree.set(tree);
    }
}

#[derive(Debug)]
pub struct Entities {
    tree: FxIndexMap<StringId, EntityTree>,
}

impl Entities {
    pub fn debug(
        &self,
        table: &ModuleTable,
        tokens: &[Spanned<LexToken>],
    ) -> FxIndexMap<String, DebugEntityTree> {
        debug_tree(&self.tree, table, tokens)
    }

    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn str_keys(&self, table: &'table ModuleTable) -> Vec<&'table str> {
        self.tree.keys().map(|id| table.lookup(id)).collect()
    }

    pub fn get_entity(&self, name: &StringId) -> Option<&Entity> {
        self.tree.get(name).map(|i| &i.entity)
    }

    pub fn get_entity_by(&self, table: &ModuleTable, name: &str) -> Entity {
        let id = table
            .get(&name)
            .expect(&format!("Entity {} didn't exist", name));

        *self
            .get_entity(&id)
            .expect(&format!("Entity {} didn't exist", name))
    }
}

fn debug_tree(
    tree: &FxIndexMap<StringId, EntityTree>,
    table: &ModuleTable,
    tokens: &[Spanned<LexToken>],
) -> FxIndexMap<String, DebugEntityTree> {
    tree.iter()
        .map(|(k, v)| (table.lookup(k).to_string(), v.debug(table, tokens)))
        .collect()
}
