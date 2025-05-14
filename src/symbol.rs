use crate::Scale;

/// trait defining the two functions all symbol types must have
pub trait SymbolTrait {
    /// minimum size of an object with the symbol at the scale
    fn min_size(&self, _scale: Scale) -> f64 {
        0.
    }

    /// the id of the symbol in the symbol_x.txt files
    fn id(&self) -> u8;
}

/// Orienteering map symbols higher order enum
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Symbol {
    /// Symbols for area objects
    Area(AreaSymbol),
    /// Symbols for line objects
    Line(LineSymbol),
    /// Symbols for point objects
    Point(PointSymbol),
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

    /// Check if symbol is a area symbol
    pub fn is_area_symbol(&self) -> bool {
        matches!(self, Symbol::Area(_))
    }

    /// get an iterator over all the symbols in draw order
    /// from bottom colors to top colors
    pub fn iter_in_draw_order() -> std::vec::IntoIter<Symbol> {
        vec![
            Symbol::Area(AreaSymbol::RoughOpenLand),
            Symbol::Area(AreaSymbol::SandyGround),
            Symbol::Area(AreaSymbol::BareRock),
            Symbol::Area(AreaSymbol::LightGreen),
            Symbol::Area(AreaSymbol::MediumGreen),
            Symbol::Area(AreaSymbol::DarkGreen),
            Symbol::Area(AreaSymbol::Marsh),
            Symbol::Area(AreaSymbol::PavedArea),
            Symbol::Line(LineSymbol::BasemapContour),
            Symbol::Line(LineSymbol::Contour),
            Symbol::Line(LineSymbol::IndexContour),
            Symbol::Line(LineSymbol::Formline),
            Symbol::Line(LineSymbol::NegBasemapContour),
            Symbol::Point(PointSymbol::SlopelineContour),
            Symbol::Point(PointSymbol::SlopelineFormline),
            Symbol::Point(PointSymbol::DotKnoll),
            Symbol::Point(PointSymbol::ElongatedDotKnoll),
            Symbol::Point(PointSymbol::UDepression),
            Symbol::Area(AreaSymbol::Water),
            Symbol::Area(AreaSymbol::GiganticBoulder),
            Symbol::Point(PointSymbol::SmallBoulder),
            Symbol::Point(PointSymbol::LargeBoulder),
            Symbol::Area(AreaSymbol::Building),
        ]
        .into_iter()
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

impl SymbolTrait for Symbol {
    fn min_size(&self, scale: Scale) -> f64 {
        match self {
            Symbol::Area(a) => a.min_size(scale),
            Symbol::Line(l) => l.min_size(scale),
            Symbol::Point(p) => p.min_size(scale),
        }
    }

    fn id(&self) -> u8 {
        match self {
            Symbol::Area(a) => a.id(),
            Symbol::Line(l) => l.id(),
            Symbol::Point(p) => p.id(),
        }
    }
}

/// Symbols for area objects
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AreaSymbol {
    RoughOpenLand,
    SandyGround,
    BareRock,
    LightGreen,
    MediumGreen,
    DarkGreen,
    Marsh,
    PavedArea,
    Water,
    GiganticBoulder,
    Building,
}

impl SymbolTrait for AreaSymbol {
    fn min_size(&self, scale: Scale) -> f64 {
        // should add min lenghts for line objects, thats why I've spelt it out
        match scale {
            Scale::S10_000 => match self {
                AreaSymbol::GiganticBoulder => 10.,
                AreaSymbol::SandyGround => 100.,
                AreaSymbol::BareRock => 100.,
                AreaSymbol::RoughOpenLand => 100.,
                AreaSymbol::LightGreen => 100.,
                AreaSymbol::MediumGreen => 50.,
                AreaSymbol::DarkGreen => 30.,
                AreaSymbol::Building => 10.,
                AreaSymbol::Water => 10.,
                AreaSymbol::PavedArea => 100.,
                AreaSymbol::Marsh => 100.,
            },
            Scale::S15_000 => match self {
                AreaSymbol::GiganticBoulder => 10.,
                AreaSymbol::SandyGround => 225.,
                AreaSymbol::BareRock => 225.,
                AreaSymbol::RoughOpenLand => 225.,
                AreaSymbol::LightGreen => 225.,
                AreaSymbol::MediumGreen => 110.,
                AreaSymbol::DarkGreen => 64.,
                AreaSymbol::Building => 10.,
                AreaSymbol::Water => 10.,
                AreaSymbol::PavedArea => 225.,
                AreaSymbol::Marsh => 225.,
            },
        }
    }

    /// get the symbol id
    fn id(&self) -> u8 {
        match self {
            AreaSymbol::GiganticBoulder => 37,
            AreaSymbol::SandyGround => 48,
            AreaSymbol::BareRock => 49,
            AreaSymbol::RoughOpenLand => 79,
            AreaSymbol::LightGreen => 83,
            AreaSymbol::MediumGreen => 86,
            AreaSymbol::DarkGreen => 90,
            AreaSymbol::Building => 140,
            AreaSymbol::Water => 51,
            AreaSymbol::PavedArea => 106,
            AreaSymbol::Marsh => 68,
        }
    }
}

/// Symbols for line objects
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LineSymbol {
    BasemapContour,
    Contour,
    IndexContour,
    Formline,
    NegBasemapContour,
}

impl SymbolTrait for LineSymbol {
    fn id(&self) -> u8 {
        match self {
            LineSymbol::Contour => 0,
            LineSymbol::BasemapContour => 2,
            LineSymbol::NegBasemapContour => unimplemented!("Not in current symbol sets"),
            LineSymbol::IndexContour => 3,
            LineSymbol::Formline => 5,
        }
    }
}

/// Symbols for point objects
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PointSymbol {
    SlopelineContour,
    SlopelineFormline,
    DotKnoll,
    ElongatedDotKnoll,
    UDepression,
    SmallBoulder,
    LargeBoulder,
}

impl SymbolTrait for PointSymbol {
    fn id(&self) -> u8 {
        match self {
            PointSymbol::SlopelineContour => 1,
            PointSymbol::SlopelineFormline => 6,
            PointSymbol::DotKnoll => 16,
            PointSymbol::ElongatedDotKnoll => 17,
            PointSymbol::UDepression => 18,
            PointSymbol::SmallBoulder => 34,
            PointSymbol::LargeBoulder => 35,
        }
    }
}
