use crate::prelude::*;

use crate::intern::Seahash;

use map::FxIndexMap;
use smart_default::SmartDefault;

#[derive(Debug, Clone, Default, new)]
pub struct ModuleTable {
    #[new(default)]
    strings: Strings,
}

impl ModuleTable {
    pub fn get(&self, hashable: &impl Seahash) -> Option<StringId> {
        self.strings.get(hashable)
    }

    pub fn lookup(&self, id: &StringId) -> &str {
        &self.strings.to_string[id.position]
    }

    pub fn intern(&mut self, hashable: &impl Seahash) -> StringId {
        self.strings.intern(hashable)
    }

    pub fn values(&self) -> Vec<String> {
        self.strings.to_string.iter().cloned().collect()
    }
}

#[derive(Debug, Clone, SmartDefault, new)]
pub struct Strings {
    #[new(default)]
    to_id: FxIndexMap<u64, StringId>,

    #[new(default)]
    to_string: Vec<String>,
}

impl Strings {
    fn get(&self, hashable: &impl Seahash) -> Option<StringId> {
        self.to_id.get(&hashable.seahash()).map(|id| *id)
    }

    crate fn intern(&mut self, hashable: &impl Seahash) -> StringId {
        if let Some(existing) = self.get(hashable) {
            existing
        } else {
            let id = StringId {
                position: self.to_string.len(),
            };

            self.to_id.insert(hashable.seahash(), id);
            self.to_string.push(hashable.to_seahashed_string());
            id
        }
    }
}
