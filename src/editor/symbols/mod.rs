mod symbol;
mod symbol_set;

use std::fmt::Display;

pub use symbol::Symbol;
pub use symbol_set::SymbolSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolType {
    Point,
    Line,
    Area,
    Text,
    Combined(CombinedSymbolType),
}

impl SymbolType {
    pub fn get_id(&self) -> usize {
        match self {
            SymbolType::Point => 1,
            SymbolType::Line => 2,
            SymbolType::Area => 4,
            SymbolType::Text => 8,
            SymbolType::Combined(_) => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombinedSymbolType {
    Area,
    Line,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolCode {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl<I> From<I> for SymbolCode
where
    I: Iterator<Item = u16>,
{
    fn from(mut value: I) -> Self {
        SymbolCode {
            major: value.next().unwrap_or(0),
            minor: value.next().unwrap_or(0),
            patch: value.next().unwrap_or(0),
        }
    }
}

impl Display for SymbolCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
