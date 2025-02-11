use strum_macros::EnumIter;
use subenum::subenum;

use crate::Scale;

// order in enum should be from bottom colors to top colors
// does not affect output but the order of writing to screen in OmapMaker
#[subenum(PointSymbol, LineSymbol, AreaSymbol)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, EnumIter)]
pub enum Symbol {
    #[subenum(AreaSymbol)]
    RoughOpenLand,
    #[subenum(AreaSymbol)]
    SandyGround,
    #[subenum(AreaSymbol)]
    BareRock,
    #[subenum(AreaSymbol)]
    LightGreen,
    #[subenum(AreaSymbol)]
    MediumGreen,
    #[subenum(AreaSymbol)]
    DarkGreen,
    #[subenum(AreaSymbol)]
    Water,
    #[subenum(LineSymbol)]
    Contour,
    #[subenum(LineSymbol)]
    BasemapContour,
    #[subenum(LineSymbol)]
    NegBasemapContour,
    #[subenum(LineSymbol)]
    IndexContour,
    #[subenum(LineSymbol)]
    Formline,
    #[subenum(PointSymbol)]
    SlopelineContour,
    #[subenum(PointSymbol)]
    SlopelineFormline,
    #[subenum(PointSymbol)]
    DotKnoll,
    #[subenum(PointSymbol)]
    ElongatedDotKnoll,
    #[subenum(PointSymbol)]
    UDepression,
    #[subenum(AreaSymbol)]
    GiganticBoulder,
    #[subenum(PointSymbol)]
    SmallBoulder,
    #[subenum(PointSymbol)]
    LargeBoulder,
    #[subenum(AreaSymbol)]
    Building,
}

impl Symbol {
    pub fn min_size(&self, scale: Scale) -> f64 {
        // should add min lenghts for line objects, thats why I've spelt it out
        match scale {
            Scale::S10_000 => match self {
                Symbol::Contour => 0.,           // Line
                Symbol::BasemapContour => 0.,    // Line
                Symbol::NegBasemapContour => 0., // Line
                Symbol::IndexContour => 0.,      // Line
                Symbol::Formline => 0.,          // Line
                Symbol::GiganticBoulder => 10.,  // Area
                Symbol::SandyGround => 100.,     // Area
                Symbol::BareRock => 100.,        // Area
                Symbol::RoughOpenLand => 100.,   // Area
                Symbol::LightGreen => 100.,      // Area
                Symbol::MediumGreen => 50.,      // Area
                Symbol::DarkGreen => 30.,        // Area
                Symbol::Building => 10.,         // Area
                Symbol::Water => 10.,            // Area
                _ => 0.,
            },
            Scale::S15_000 => match self {
                Symbol::Contour => 0.,           // Line
                Symbol::BasemapContour => 0.,    // Line
                Symbol::NegBasemapContour => 0., // Line
                Symbol::IndexContour => 0.,      // Line
                Symbol::Formline => 0.,          // Line
                Symbol::GiganticBoulder => 10.,  // Area
                Symbol::SandyGround => 225.,     // Area
                Symbol::BareRock => 225.,        // Area
                Symbol::RoughOpenLand => 225.,   // Area
                Symbol::LightGreen => 225.,      // Area
                Symbol::MediumGreen => 110.,     // Area
                Symbol::DarkGreen => 64.,        // Area
                Symbol::Building => 10.,         // Area
                Symbol::Water => 10.,            // Area
                _ => 0.,
            },
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Symbol::Contour => 0,
            Symbol::BasemapContour => 2,
            Symbol::NegBasemapContour => unimplemented!("Not in current symbol sets"),
            Symbol::IndexContour => 3,
            Symbol::Formline => 5,
            Symbol::GiganticBoulder => 37,
            Symbol::SandyGround => 48,
            Symbol::BareRock => 49,
            Symbol::RoughOpenLand => 79,
            Symbol::LightGreen => 83,
            Symbol::MediumGreen => 86,
            Symbol::DarkGreen => 90,
            Symbol::Building => 140,
            Symbol::Water => 51,
            Symbol::SlopelineContour => 1,
            Symbol::SlopelineFormline => 6,
            Symbol::DotKnoll => 16,
            Symbol::ElongatedDotKnoll => 17,
            Symbol::UDepression => 18,
            Symbol::SmallBoulder => 34,
            Symbol::LargeBoulder => 35,
        }
    }
}

impl LineSymbol {
    pub fn id(&self) -> u8 {
        (Symbol::from(*self)).id()
    }

    pub fn min_size(&self, scale: Scale) -> f64 {
        (Symbol::from(*self)).min_size(scale)
    }
}

impl PointSymbol {
    pub fn id(&self) -> u8 {
        (Symbol::from(*self)).id()
    }
}

impl AreaSymbol {
    pub fn id(&self) -> u8 {
        (Symbol::from(*self)).id()
    }

    pub fn min_size(&self, scale: Scale) -> f64 {
        (Symbol::from(*self)).min_size(scale)
    }
}
