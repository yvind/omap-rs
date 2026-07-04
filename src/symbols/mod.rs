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
            _ => Err(Error::SymbolConversionError),
        }
    }
}

impl From<Weak<RefCell<LineSymbol>>> for WeakLinePathSymbol {
    fn from(value: Weak<RefCell<LineSymbol>>) -> Self {
        WeakLinePathSymbol::Line(value)
    }
}

impl From<Weak<RefCell<CombinedLineSymbol>>> for WeakLinePathSymbol {
    fn from(value: Weak<RefCell<CombinedLineSymbol>>) -> Self {
        WeakLinePathSymbol::CombinedLine(value)
    }
}

impl From<WeakLinePathSymbol> for WeakSymbol {
    fn from(value: WeakLinePathSymbol) -> Self {
        match value {
            WeakLinePathSymbol::Line(weak) => WeakSymbol::Line(weak),
            WeakLinePathSymbol::CombinedLine(weak) => WeakSymbol::CombinedLine(weak),
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
            _ => Err(Error::SymbolConversionError),
        }
    }
}

impl From<Weak<RefCell<AreaSymbol>>> for WeakAreaPathSymbol {
    fn from(value: Weak<RefCell<AreaSymbol>>) -> Self {
        WeakAreaPathSymbol::Area(value)
    }
}

impl From<Weak<RefCell<CombinedAreaSymbol>>> for WeakAreaPathSymbol {
    fn from(value: Weak<RefCell<CombinedAreaSymbol>>) -> Self {
        WeakAreaPathSymbol::CombinedArea(value)
    }
}

impl From<WeakAreaPathSymbol> for WeakSymbol {
    fn from(value: WeakAreaPathSymbol) -> Self {
        match value {
            WeakAreaPathSymbol::Area(weak) => WeakSymbol::Area(weak),
            WeakAreaPathSymbol::CombinedArea(weak) => WeakSymbol::CombinedArea(weak),
        }
    }
}

/// A combined-symbol part that is either a public (shared) reference or a private (embedded) symbol.
#[derive(Debug, Clone)]
pub enum PublicOrPrivateSymbol<W: std::fmt::Debug + Clone, P: std::fmt::Debug + Clone> {
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
            _ => Err(Error::SymbolConversionError),
        }
    }
}

impl TryFrom<WeakPathSymbol> for WeakAreaPathSymbol {
    type Error = Error;

    fn try_from(value: WeakPathSymbol) -> Result<Self> {
        match value {
            WeakPathSymbol::Area(ref_cell) => Ok(WeakAreaPathSymbol::Area(ref_cell)),
            WeakPathSymbol::CombinedArea(ref_cell) => {
                Ok(WeakAreaPathSymbol::CombinedArea(ref_cell))
            }
            _ => Err(Error::SymbolConversionError),
        }
    }
}

impl TryFrom<WeakPathSymbol> for WeakLinePathSymbol {
    type Error = Error;

    fn try_from(value: WeakPathSymbol) -> Result<Self> {
        match value {
            WeakPathSymbol::Line(ref_cell) => Ok(WeakLinePathSymbol::Line(ref_cell)),
            WeakPathSymbol::CombinedLine(ref_cell) => {
                Ok(WeakLinePathSymbol::CombinedLine(ref_cell))
            }
            _ => Err(Error::SymbolConversionError),
        }
    }
}

impl From<WeakPathSymbol> for WeakSymbol {
    fn from(value: WeakPathSymbol) -> Self {
        match value {
            WeakPathSymbol::Area(weak) => WeakSymbol::Area(weak),
            WeakPathSymbol::Line(weak) => WeakSymbol::Line(weak),
            WeakPathSymbol::CombinedArea(weak) => WeakSymbol::CombinedArea(weak),
            WeakPathSymbol::CombinedLine(weak) => WeakSymbol::CombinedLine(weak),
        }
    }
}

impl From<WeakAreaPathSymbol> for WeakPathSymbol {
    fn from(value: WeakAreaPathSymbol) -> Self {
        match value {
            WeakAreaPathSymbol::Area(weak) => WeakPathSymbol::Area(weak),
            WeakAreaPathSymbol::CombinedArea(weak) => WeakPathSymbol::CombinedArea(weak),
        }
    }
}

impl From<WeakLinePathSymbol> for WeakPathSymbol {
    fn from(value: WeakLinePathSymbol) -> Self {
        match value {
            WeakLinePathSymbol::Line(weak) => WeakPathSymbol::Line(weak),
            WeakLinePathSymbol::CombinedLine(weak) => WeakPathSymbol::CombinedLine(weak),
        }
    }
}

macro_rules! impl_from_weak_path_symbol {
    ($symbol_ty:ty, $variant:ident) => {
        impl From<Weak<RefCell<$symbol_ty>>> for WeakPathSymbol {
            fn from(value: Weak<RefCell<$symbol_ty>>) -> Self {
                WeakPathSymbol::$variant(value)
            }
        }
    };
}

impl_from_weak_path_symbol!(AreaSymbol, Area);
impl_from_weak_path_symbol!(CombinedAreaSymbol, CombinedArea);
impl_from_weak_path_symbol!(LineSymbol, Line);
impl_from_weak_path_symbol!(CombinedLineSymbol, CombinedLine);
