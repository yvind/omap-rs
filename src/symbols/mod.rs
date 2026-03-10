mod area_symbol;
mod combined_area_symbol;
mod combined_line_symbol;
mod line_symbol;
mod point_symbol;
mod symbol;
mod symbol_set;
mod text_symbol;

use std::{cell::RefCell, rc::Weak};

pub use area_symbol::AreaSymbol;
pub use combined_area_symbol::CombinedAreaSymbol;
pub use combined_line_symbol::CombinedLineSymbol;
pub use line_symbol::LineSymbol;
pub use point_symbol::PointSymbol;
pub use symbol::{Symbol, SymbolCommon, WeakSymbol};
pub use symbol_set::SymbolSet;
pub use text_symbol::TextSymbol;

/// The symbol used to render a line object.
#[derive(Debug, Clone)]
pub enum LineObjectSymbol {
    /// A standalone line symbol.
    Line(Weak<RefCell<LineSymbol>>),
    /// A combined line symbol.
    CombinedLine(Weak<RefCell<CombinedLineSymbol>>),
}

/// The symbol used to render an area object.
#[derive(Debug, Clone)]
pub enum AreaObjectSymbol {
    /// A standalone area symbol.
    Area(Weak<RefCell<AreaSymbol>>),
    /// A combined area symbol.
    CombinedArea(Weak<RefCell<CombinedAreaSymbol>>),
}

/// A combined-symbol part that is either a public (shared) reference or a private (embedded) symbol.
#[derive(Debug, Clone)]
pub enum PubOrPrivSymbol<W: std::fmt::Debug + Clone, P: std::fmt::Debug + Clone> {
    /// A public (shared) reference to another symbol in the symbol set.
    Public(W),
    /// A private (embedded) sub-symbol.
    Private(P),
}
