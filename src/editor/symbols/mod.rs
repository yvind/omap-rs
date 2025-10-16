mod symbol;
mod symbol_set;

pub use symbol::Symbol;
pub use symbol_set::SymbolSet;

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
