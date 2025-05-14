/// Map scale
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    /// 1:10_000
    S10_000,
    /// 1:15_000
    S15_000,
}

use std::fmt;
impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scale::S10_000 => write!(f, "10000"),
            Scale::S15_000 => write!(f, "15000"),
        }
    }
}
