use std::fmt::Debug;

pub trait Macro: Debug {
    fn annotate(&self) -> Vec<Box<dyn Term>>;
}

pub trait Term: Debug {}
