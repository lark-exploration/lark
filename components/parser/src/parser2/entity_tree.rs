use crate::parser2::token_tree::{TokenPos, TokenSpan};
use crate::StringId;

use derive_new::new;
use std::collections::BTreeMap;

#[derive(Debug, Clone, new)]
pub struct EntityTreeBuilder {
    #[new(value = "vec![]")]
    stack: Vec<(StringId, TokenPos)>,

    #[new(value = "BTreeMap::new()")]
    current: BTreeMap<StringId, Entity>,
}

#[derive(Debug, Clone)]
pub struct EntityTree(BTreeMap<StringId, Entity>);

#[derive(Debug, Clone)]
pub struct Entity {
    name: StringId,
    tokens: TokenSpan,
    children: BTreeMap<StringId, Entity>,
}

impl EntityTreeBuilder {
    pub fn push(&mut self, name: StringId, pos: TokenPos) {
        self.stack.push((name, pos));
    }

    pub fn finish(&mut self, end: TokenPos) {
        let EntityTreeBuilder { stack, current } = self;

        let mut parent = BTreeMap::new();
        let (name, start) = stack.pop().expect("Can't finish an unopened entity");

        let tokens = TokenSpan(start, end);

        parent.insert(
            name,
            Entity {
                name,
                tokens,
                children: current.clone(),
            },
        );

        self.current = parent;
    }

    pub fn finalize(self) -> EntityTree {
        assert!(
            self.stack.len() == 0,
            "Can only finalize an entity tree when nothing is on the stack, stack={:?}",
            self.stack
        );

        EntityTree(self.current)
    }
}
