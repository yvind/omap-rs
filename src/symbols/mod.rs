mod area_symbol;
mod combined_area_symbol;
mod combined_line_symbol;
mod ids;
mod line_symbol;
mod point_symbol;
mod symbol;
mod symbol_set;
mod text_symbol;

pub use area_symbol::{AreaSymbol, ClippingOption, FillPattern};
pub use combined_area_symbol::CombinedAreaSymbol;
pub use combined_line_symbol::CombinedLineSymbol;
pub use ids::{
    AreaPathSymbolId, AreaSymbolId, CombinedAreaSymbolId, CombinedLineSymbolId, LinePathSymbolId,
    LineSymbolId, PathSymbolId, PointSymbolId, SymbolId, TextSymbolId,
};
pub use line_symbol::{
    BorderDash, BorderStyle, CapStyle, DashStyle, DashSymbol, GroupDashes, JoinStyle, LineSymbol,
    LineSymbolBorder, MidSymbol, MidSymbolPlacement,
};
pub use point_symbol::{Element, PointSymbol};
pub use symbol::{Symbol, SymbolCommon};
pub use symbol_set::SymbolSet;
pub use text_symbol::{FramingMode, LineBelow, LineFraming, ShadowFraming, TextSymbol};

/// A combined-symbol part that is either a public (shared) reference or a private (embedded) symbol.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PublicOrPrivateSymbol<W, P> {
    /// A public (shared) reference to another symbol in the symbol set.
    Public(W),
    /// A private (embedded) sub-symbol.
    Private(P),
}

/// An area or line symbol used in private parts of area combined symbols
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AreaOrLineSymbol {
    /// An area sub-symbol.
    Area(Box<AreaSymbol>),
    /// A line sub-symbol.
    Line(Box<LineSymbol>),
}

macro_rules! impl_from_area_or_line_symbol {
    ($symbol_ty:ty, $variant:ident) => {
        impl From<$symbol_ty> for AreaOrLineSymbol {
            fn from(value: $symbol_ty) -> Self {
                AreaOrLineSymbol::$variant(Box::new(value))
            }
        }

        impl From<Box<$symbol_ty>> for AreaOrLineSymbol {
            fn from(value: Box<$symbol_ty>) -> Self {
                AreaOrLineSymbol::$variant(value)
            }
        }
    };
}

impl_from_area_or_line_symbol!(AreaSymbol, Area);
impl_from_area_or_line_symbol!(LineSymbol, Line);
