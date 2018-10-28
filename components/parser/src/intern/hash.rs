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
