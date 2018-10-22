pub const ALLOW_NEWLINE: AllowPolicy = AllowPolicy(0b0001);
pub const ALLOW_EOF: AllowPolicy = AllowPolicy(0b0010);
pub const ALLOW_NONE: AllowPolicy = AllowPolicy(0b0000);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AllowPolicy(u8);

impl std::ops::BitOr for AllowPolicy {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        AllowPolicy(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for AllowPolicy {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        AllowPolicy(self.0 & rhs.0)
    }
}

impl AllowPolicy {
    pub fn has(&self, policy: AllowPolicy) -> bool {
        (self.0 & policy.0) != 0
    }

    pub fn include_newline(&self) -> bool {
        *self == ALLOW_NEWLINE
    }
}
