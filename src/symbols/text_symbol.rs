use super::SymbolTrait;

/// Symbols for text objects
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextSymbol {
    ContourValue,
    SpotHeight,
    ControlNumber,
}

impl SymbolTrait for TextSymbol {
    fn id(&self) -> u8 {
        match self {
            TextSymbol::ContourValue => 5,
            TextSymbol::SpotHeight => 164,
            TextSymbol::ControlNumber => 165,
        }
    }
}
