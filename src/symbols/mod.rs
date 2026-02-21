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

#[derive(Debug, Clone)]
pub enum LineObjectSymbol {
    Line(Weak<RefCell<LineSymbol>>),
    CombinedLine(Weak<RefCell<CombinedLineSymbol>>),
}

#[derive(Debug, Clone)]
pub enum AreaObjectSymbol {
    Area(Weak<RefCell<AreaSymbol>>),
    CombinedArea(Weak<RefCell<CombinedAreaSymbol>>),
}

#[derive(Debug, Clone)]
pub enum PubOrPrivSymbol<W: std::fmt::Debug + Clone, P: std::fmt::Debug + Clone> {
    Public(W),
    Private(P),
}
