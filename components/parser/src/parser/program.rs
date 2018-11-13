use crate::prelude::*;

use derive_new::new;
use smart_default::SmartDefault;
use std::collections::BTreeMap;

#[derive(Copy, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NameId {
    position: usize,
}

#[derive(Debug, Clone, SmartDefault, new)]
pub struct Environment<'parent> {
    #[new(default)]
    #[default = "BTreeMap::new()"]
    to_name: BTreeMap<GlobalIdentifier, NameId>,

    #[new(default)]
    #[default = "None"]
    parent: Option<&'parent Environment<'parent>>,
}
