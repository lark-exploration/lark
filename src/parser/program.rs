use codespan::CodeMap;
use crate::parser::Spanned;
use derive_new::new;
use smart_default::SmartDefault;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StringId {
    position: usize,
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
    to_string: Vec<Spanned<String>>,
}

impl Strings {
    fn get(&self, hashable: &impl Seahash) -> Option<StringId> {
        self.to_id.get(&hashable.seahash()).map(|id| *id)
    }

    crate fn intern(&mut self, hashable: impl Seahash) -> StringId {
        if let Some(existing) = self.get(&hashable) {
            existing
        } else {
            let id = StringId {
                position: self.to_string.len(),
            };

            self.to_id.insert(hashable.seahash(), id);
            self.to_string.push(hashable.into_spanned_string());
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
    crate fn get(&self, hashable: impl Seahash) -> Option<StringId> {
        self.strings.get(&hashable)
    }

    crate fn lookup(&self, id: &StringId) -> &str {
        &self.strings.to_string[id.position]
    }

    crate fn intern(&mut self, hashable: impl Seahash) -> StringId {
        self.strings.intern(hashable)
    }

    crate fn values(&self) -> Vec<String> {
        self.strings
            .to_string
            .iter()
            .cloned()
            .map(|s| s.0)
            .collect()
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

    crate fn get_str(&self, program: &ModuleTable, key: impl Seahash) -> Option<NameId> {
        let id = program.get(key)?;

        self.get(id)
    }
}

crate trait Seahash: Into<String> {
    fn seahash(&self) -> u64;
    fn into_spanned_string(self) -> Spanned<String> {
        Spanned::synthetic(self.into())
    }
}

impl Seahash for String {
    fn seahash(&self) -> u64 {
        seahash::hash(self.as_bytes())
    }

    fn into_spanned_string(self) -> Spanned<String> {
        Spanned::synthetic(self)
    }
}

impl Seahash for &str {
    fn seahash(&self) -> u64 {
        seahash::hash(self.as_bytes())
    }
}

impl Into<String> for Spanned<String> {
    fn into(self) -> String {
        self.0
    }
}

impl Seahash for Spanned<String> {
    fn seahash(&self) -> u64 {
        seahash::hash(self.0.as_bytes())
    }

    fn into_spanned_string(self) -> Spanned<String> {
        self
    }
}

pub struct Module<'table> {
    table: &'table mut ModuleTable,
    names: BTreeMap<NameId, StringId>,
}
