mod symbol;
mod symbol_set;

pub use symbol::Symbol;
pub use symbol_set::SymbolSet;

pub type SymbolId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolType {
    Area,
    Line,
    Point,
    Text,
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
