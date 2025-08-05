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

impl std::str::FromStr for SymbolType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "area" | "area_symbol" => Ok(SymbolType::Area),
            "line" | "line_symbol" => Ok(SymbolType::Line),
            "point" | "point_symbol" => Ok(SymbolType::Point),
            "text" | "text_symbol" => Ok(SymbolType::Text),
            _ => Err(format!("Unknown symbol type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolCode {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl From<(u16, u16, u16)> for SymbolCode {
    fn from(value: (u16, u16, u16)) -> Self {
        SymbolCode {
            major: value.0,
            minor: value.1,
            patch: value.2,
        }
    }
}
