mod symbol;
mod symbol_set;

use std::{fmt::Display, num::ParseIntError, str::FromStr};

pub use symbol::Symbol;
pub use symbol_set::SymbolSet;

#[derive(Debug, Clone, Copy)]
pub struct SymbolId(usize);

impl Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SymbolId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SymbolId(usize::from_str(s)?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolType {
    Point = 1,
    Line = 2,
    Area = 4,
    Text = 8,
    Combined = 16,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolCode {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl From<[u16; 3]> for SymbolCode {
    fn from(value: [u16; 3]) -> Self {
        SymbolCode {
            major: value[0],
            minor: value[1],
            patch: value[2],
        }
    }
}
