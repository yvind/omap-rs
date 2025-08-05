use super::{AreaSymbol, LineSymbol, PointSymbol, SymbolTrait, TextSymbol};
use crate::writer::Scale;
use std::fmt;

/// Orienteering map symbols higher order enum
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Symbol {
    /// Symbols for area objects
    Area(AreaSymbol),
    /// Symbols for line objects
    Line(LineSymbol),
    /// Symbols for point objects
    Point(PointSymbol),
    /// Symbols for text objects
    Text(TextSymbol),
}

impl Symbol {
    /// Check if symbol is a line symbol
    pub fn is_line_symbol(&self) -> bool {
        matches!(self, Symbol::Line(_))
    }

    /// Check if symbol is a point symbol
    pub fn is_point_symbol(&self) -> bool {
        matches!(self, Symbol::Point(_))
    }

    /// Check if symbol is an area symbol
    pub fn is_area_symbol(&self) -> bool {
        matches!(self, Symbol::Area(_))
    }

    /// Check if symbol is a text symbol
    pub fn is_text_symbol(&self) -> bool {
        matches!(self, Symbol::Text(_))
    }

    pub(crate) fn is_not_bezier_symbol(&self) -> bool {
        match self {
            Symbol::Line(line_symbol) => matches!(
                line_symbol,
                LineSymbol::BasemapContour | LineSymbol::NegBasemapContour
            ),
            Symbol::Area(area_symbol) => matches!(
                area_symbol,
                AreaSymbol::CanopyWithOutline
                    | AreaSymbol::CanopyWithoutOutline
                    | AreaSymbol::Building
                    | AreaSymbol::LargeBuildingWithOutline
                    | AreaSymbol::LargeBuildingWithoutOutline
            ),
            _ => true,
        }
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Symbol::Area(area_symbol) => write!(f, "{:?}", area_symbol),
            Symbol::Line(line_symbol) => write!(f, "{:?}", line_symbol),
            Symbol::Point(point_symbol) => write!(f, "{:?}", point_symbol),
            Symbol::Text(text_symbol) => write!(f, "{:?}", text_symbol),
        }
    }
}

impl From<AreaSymbol> for Symbol {
    fn from(value: AreaSymbol) -> Self {
        Symbol::Area(value)
    }
}

impl From<LineSymbol> for Symbol {
    fn from(value: LineSymbol) -> Self {
        Symbol::Line(value)
    }
}

impl From<PointSymbol> for Symbol {
    fn from(value: PointSymbol) -> Self {
        Symbol::Point(value)
    }
}

impl From<TextSymbol> for Symbol {
    fn from(value: TextSymbol) -> Self {
        Symbol::Text(value)
    }
}

impl SymbolTrait for Symbol {
    fn min_size(&self, scale: Scale) -> f64 {
        match self {
            Symbol::Area(a) => a.min_size(scale),
            Symbol::Line(l) => l.min_size(scale),
            Symbol::Point(p) => p.min_size(scale),
            Symbol::Text(t) => t.min_size(scale),
        }
    }

    fn id(&self) -> u8 {
        match self {
            Symbol::Area(a) => a.id(),
            Symbol::Line(l) => l.id(),
            Symbol::Point(p) => p.id(),
            Symbol::Text(t) => t.id(),
        }
    }

    fn is_rotatable(&self) -> bool {
        match self {
            Symbol::Area(a) => a.is_rotatable(),
            Symbol::Line(l) => l.is_rotatable(),
            Symbol::Point(p) => p.is_rotatable(),
            Symbol::Text(t) => t.is_rotatable(),
        }
    }
}
