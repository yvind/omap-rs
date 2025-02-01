use crate::Scale;

#[derive(Copy, Clone)]
pub enum LineSymbol {
    Contour,
    BasemapContour,
    IndexContour,
    Formline,
}

#[derive(Copy, Clone)]
pub enum AreaSymbol {
    GiganticBoulder,
    SandyGround,
    BareRock,
    RoughOpenLand,
    LightGreen,
    MediumGreen,
    DarkGreen,
    Building,
    Water,
}

#[derive(Copy, Clone)]
pub enum PointSymbol {
    SlopelineContour,
    SlopelineFormline,
    DotKnoll,
    ElongatedDotKnoll,
    UDepression,
    SmallBoulder,
    LargeBoulder,
}

pub trait Symbol {
    fn min_size(&self, scale: Scale) -> f64;
    fn id(&self) -> u8;
}

impl Symbol for LineSymbol {
    fn min_size(&self, scale: Scale) -> f64 {
        // should add min lenghts for line objects
        match scale {
            Scale::S10_000 => match self {
                LineSymbol::Contour => 0.,
                LineSymbol::BasemapContour => 0.,
                LineSymbol::IndexContour => 0.,
                LineSymbol::Formline => 0.,
            },
            Scale::S15_000 => match self {
                LineSymbol::Contour => 0.,
                LineSymbol::BasemapContour => 0.,
                LineSymbol::IndexContour => 0.,
                LineSymbol::Formline => 0.,
            },
        }
    }

    fn id(&self) -> u8 {
        match self {
            LineSymbol::Contour => 0,
            LineSymbol::BasemapContour => 2,
            LineSymbol::IndexContour => 3,
            LineSymbol::Formline => 5,
        }
    }
}

impl Symbol for AreaSymbol {
    fn min_size(&self, scale: Scale) -> f64 {
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
            },
        }
    }

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
        }
    }
}

impl Symbol for PointSymbol {
    fn min_size(&self, _scale: Scale) -> f64 {
        0.
    }

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
