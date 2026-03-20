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

use crate::{Error, Result};

/// The symbol used to render a line object.
#[derive(Debug, Clone)]
pub enum WeakLinePathSymbol {
    /// A standalone line symbol.
    Line(Weak<RefCell<LineSymbol>>),
    /// A combined line symbol.
    CombinedLine(Weak<RefCell<CombinedLineSymbol>>),
}

impl WeakLinePathSymbol {
    /// Attempt to upgrade the weak reference to a strong [`Symbol`].
    pub fn upgrade(&self) -> Option<Symbol> {
        match self {
            WeakLinePathSymbol::Line(weak) => weak.upgrade().map(Symbol::Line),
            WeakLinePathSymbol::CombinedLine(weak) => weak.upgrade().map(Symbol::CombinedLine),
        }
    }
}

impl TryFrom<WeakSymbol> for WeakLinePathSymbol {
    type Error = Error;

    fn try_from(value: WeakSymbol) -> Result<Self> {
        match value {
            WeakSymbol::Line(ref_cell) => Ok(WeakLinePathSymbol::Line(ref_cell)),
            WeakSymbol::CombinedLine(ref_cell) => Ok(WeakLinePathSymbol::CombinedLine(ref_cell)),
            _ => Err(Error::SymbolError(
                "Cannot only convert weak line and combined line to WeakLinePathSymbol".to_string(),
            )),
        }
    }
}

/// The symbol used to render an area object.
#[derive(Debug, Clone)]
pub enum WeakAreaPathSymbol {
    /// A standalone area symbol.
    Area(Weak<RefCell<AreaSymbol>>),
    /// A combined area symbol.
    CombinedArea(Weak<RefCell<CombinedAreaSymbol>>),
}

impl WeakAreaPathSymbol {
    /// Attempt to upgrade the weak reference to a strong [`Symbol`].
    pub fn upgrade(&self) -> Option<Symbol> {
        match self {
            WeakAreaPathSymbol::Area(weak) => weak.upgrade().map(Symbol::Area),
            WeakAreaPathSymbol::CombinedArea(weak) => weak.upgrade().map(Symbol::CombinedArea),
        }
    }
}

impl TryFrom<WeakSymbol> for WeakAreaPathSymbol {
    type Error = Error;

    fn try_from(value: WeakSymbol) -> Result<Self> {
        match value {
            WeakSymbol::Area(ref_cell) => Ok(WeakAreaPathSymbol::Area(ref_cell)),
            WeakSymbol::CombinedArea(ref_cell) => Ok(WeakAreaPathSymbol::CombinedArea(ref_cell)),
            _ => Err(Error::SymbolError(
                "Cannot only convert weak area and combined area to WeakAreaPathSymbol".to_string(),
            )),
        }
    }
}

/// A combined-symbol part that is either a public (shared) reference or a private (embedded) symbol.
#[derive(Debug, Clone)]
pub enum PubOrPrivSymbol<W: std::fmt::Debug + Clone, P: std::fmt::Debug + Clone> {
    /// A public (shared) reference to another symbol in the symbol set.
    Public(W),
    /// A private (embedded) sub-symbol.
    Private(P),
}

/// An area or line symbol used in private parts of area combined symbols
#[derive(Debug, Clone)]
pub enum AreaOrLineSymbol {
    /// An area sub-symbol.
    Area(Box<AreaSymbol>),
    /// A line sub-symbol.
    Line(Box<LineSymbol>),
}

/// A non-owning reference to a area or line symbol, used in public parts of area combined symbols
#[derive(Debug, Clone)]
pub enum WeakPathSymbol {
    /// A weak reference to an area symbol.
    Area(Weak<RefCell<AreaSymbol>>),
    /// A weak reference to a line symbol.
    Line(Weak<RefCell<LineSymbol>>),
    /// A weak reference to a combined area symbol.
    CombinedArea(Weak<RefCell<CombinedAreaSymbol>>),
    /// A weak reference to a combined line symbol.
    CombinedLine(Weak<RefCell<CombinedLineSymbol>>),
}

impl WeakPathSymbol {
    /// Attempt to upgrade the weak reference to a strong [`Symbol`].
    pub fn upgrade(&self) -> Option<Symbol> {
        match self {
            WeakPathSymbol::Area(weak) => weak.upgrade().map(Symbol::Area),
            WeakPathSymbol::Line(weak) => weak.upgrade().map(Symbol::Line),
            WeakPathSymbol::CombinedArea(weak) => weak.upgrade().map(Symbol::CombinedArea),
            WeakPathSymbol::CombinedLine(weak) => weak.upgrade().map(Symbol::CombinedLine),
        }
    }
}

impl TryFrom<WeakSymbol> for WeakPathSymbol {
    type Error = Error;

    fn try_from(value: WeakSymbol) -> Result<Self> {
        match value {
            WeakSymbol::Line(ref_cell) => Ok(WeakPathSymbol::Line(ref_cell)),
            WeakSymbol::Area(ref_cell) => Ok(WeakPathSymbol::Area(ref_cell)),
            WeakSymbol::CombinedArea(ref_cell) => Ok(WeakPathSymbol::CombinedArea(ref_cell)),
            WeakSymbol::CombinedLine(ref_cell) => Ok(WeakPathSymbol::CombinedLine(ref_cell)),
            _ => Err(Error::SymbolError(
                "Cannot convert Text or Line weak symbol to WeakPathSymbol".to_string(),
            )),
        }
    }
}
