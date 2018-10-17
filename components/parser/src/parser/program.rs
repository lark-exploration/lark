use crate::parser::Spanned;

use codespan::CodeMap;
use debug::DebugWith;
use derive_new::new;
use smart_default::SmartDefault;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StringId {
    position: usize,
}

debug::debug_fallback_impl!(StringId);

pub trait LookupStringId {
    fn lookup(&self, id: StringId) -> Arc<String>;
}

impl<Cx: LookupStringId> DebugWith<Cx> for StringId {
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&cx.lookup(*self), fmt)
    }
}

impl fmt::Debug for StringId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.position)
    }
}

#[derive(Copy, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NameId {
    position: usize,
}

#[derive(Debug, Clone, SmartDefault, new)]
pub struct Strings {
    #[new(default)]
    #[default = "BTreeMap::new()"]
    to_id: BTreeMap<u64, StringId>,

    #[new(default)]
    #[default = "vec![]"]
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
pub struct Environment<'parent> {
    #[new(default)]
    #[default = "BTreeMap::new()"]
    to_name: BTreeMap<StringId, NameId>,

    #[new(default)]
    #[default = "None"]
    parent: Option<&'parent Environment<'parent>>,
}

impl Environment<'parent> {
    crate fn child(&'current self) -> Environment<'current> {
        Environment {
            to_name: BTreeMap::new(),
            parent: Some(self),
        }
    }

    crate fn bind(&mut self, name: StringId, value: NameId) {
        self.to_name.insert(name, value);
    }

    crate fn get(&self, name: StringId) -> Option<NameId> {
        self.to_name.get(&name).map(|id| *id)
    }

    crate fn get_str(&self, program: &ModuleTable, key: &impl Seahash) -> Option<NameId> {
        let id = program.get(key)?;

        self.get(id)
    }
}

pub trait Seahash {
    fn seahash(&self) -> u64;
    fn to_seahashed_string(&self) -> String;
}

impl Seahash for &str {
    fn seahash(&self) -> u64 {
        seahash::hash(self.as_bytes())
    }

    fn to_seahashed_string(&self) -> String {
        self.to_string()
    }
}

pub struct Module<'table> {
    table: &'table mut ModuleTable,
    names: BTreeMap<NameId, StringId>,
}
