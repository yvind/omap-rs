use geo_types::{Error, LineString, Point, Polygon};

use crate::Scale;

#[derive(Clone, Debug)]
pub enum Symbol {
    Contour(LineString),
    SlopelineContour(Point, f64),
    BasemapContour(LineString),
    IndexContour(LineString),
    Formline(LineString),
    SlopelineFormline(Point, f64),
    SmallBoulder(Point),
    LargeBoulder(Point),
    GiganticBoulder(Point),
    SandyGround(Polygon),
    BareRock(Polygon),
    RoughOpenLand(Polygon),
    LightGreen(Polygon),
    MediumGreen(Polygon),
    DarkGreen(Polygon),
    Building(Polygon),
}

impl Symbol {
    pub fn min_size(&self, scale: Scale) -> f64 {
        match scale {
            Scale::S10_000 => 0.,
            Scale::S15_000 => match self {
                Symbol::Contour(_) => 0.,
                Symbol::SlopelineContour(_, _) => 0.,
                Symbol::BasemapContour(_) => 0.,
                Symbol::IndexContour(_) => 0.,
                Symbol::Formline(_) => 0.,
                Symbol::SlopelineFormline(_, _) => 0.,
                Symbol::SmallBoulder(_) => 0.,
                Symbol::LargeBoulder(_) => 0.,
                Symbol::GiganticBoulder(_) => 10.,
                Symbol::SandyGround(_) => 225.,
                Symbol::BareRock(_) => 225.,
                Symbol::RoughOpenLand(_) => 225.,
                Symbol::LightGreen(_) => 225.,
                Symbol::MediumGreen(_) => 110.,
                Symbol::DarkGreen(_) => 64.,
                Symbol::Building(_) => 0.,
            },
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Symbol::Contour(_) => 0,
            Symbol::SlopelineContour(_, _) => 1,
            Symbol::BasemapContour(_) => 2,
            Symbol::IndexContour(_) => 3,
            Symbol::Formline(_) => 5,
            Symbol::SlopelineFormline(_, _) => 6,
            Symbol::SmallBoulder(_) => 34,
            Symbol::LargeBoulder(_) => 35,
            Symbol::GiganticBoulder(_) => 37,
            Symbol::SandyGround(_) => 48,
            Symbol::BareRock(_) => 49,
            Symbol::RoughOpenLand(_) => 79,
            Symbol::LightGreen(_) => 83,
            Symbol::MediumGreen(_) => 86,
            Symbol::DarkGreen(_) => 90,
            Symbol::Building(_) => 140,
        }
    }

    pub fn rotation(&self) -> f64 {
        match self {
            Symbol::Contour(_) => 0.,
            Symbol::SlopelineContour(_, rot) => *rot,
            Symbol::BasemapContour(_) => 0.,
            Symbol::IndexContour(_) => 0.,
            Symbol::Formline(_) => 0.,
            Symbol::SlopelineFormline(_, rot) => *rot,
            Symbol::SmallBoulder(_) => 0.,
            Symbol::LargeBoulder(_) => 0.,
            Symbol::GiganticBoulder(_) => 0.,
            Symbol::SandyGround(_) => 0.,
            Symbol::BareRock(_) => 0.,
            Symbol::RoughOpenLand(_) => 0.,
            Symbol::LightGreen(_) => 0.,
            Symbol::MediumGreen(_) => 0.,
            Symbol::DarkGreen(_) => 0.,
            Symbol::Building(_) => 0.,
        }
    }

    pub fn num_coords(&self) -> usize {
        match self {
            Symbol::Contour(line_string) => line_string.0.len(),
            Symbol::SlopelineContour(_, _) => 1,
            Symbol::BasemapContour(line_string) => line_string.0.len(),
            Symbol::IndexContour(line_string) => line_string.0.len(),
            Symbol::Formline(line_string) => line_string.0.len(),
            Symbol::SlopelineFormline(_, _) => 1,
            Symbol::SmallBoulder(_) => 1,
            Symbol::LargeBoulder(_) => 1,
            Symbol::GiganticBoulder(_) => 1,
            Symbol::SandyGround(polygon) => {
                polygon.exterior().0.len()
                    + polygon.interiors().iter().fold(0, |acc, l| acc + l.0.len())
            }
            Symbol::BareRock(polygon) => {
                polygon.exterior().0.len()
                    + polygon.interiors().iter().fold(0, |acc, l| acc + l.0.len())
            }
            Symbol::RoughOpenLand(polygon) => {
                polygon.exterior().0.len()
                    + polygon.interiors().iter().fold(0, |acc, l| acc + l.0.len())
            }
            Symbol::LightGreen(polygon) => {
                polygon.exterior().0.len()
                    + polygon.interiors().iter().fold(0, |acc, l| acc + l.0.len())
            }
            Symbol::MediumGreen(polygon) => {
                polygon.exterior().0.len()
                    + polygon.interiors().iter().fold(0, |acc, l| acc + l.0.len())
            }
            Symbol::DarkGreen(polygon) => {
                polygon.exterior().0.len()
                    + polygon.interiors().iter().fold(0, |acc, l| acc + l.0.len())
            }
            Symbol::Building(polygon) => {
                polygon.exterior().0.len()
                    + polygon.interiors().iter().fold(0, |acc, l| acc + l.0.len())
            }
        }
    }
}

impl TryFrom<Symbol> for Point {
    type Error = crate::OmapError;

    fn try_from(value: Symbol) -> Result<Point, Self::Error> {
        match value {
            Symbol::Contour(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "LineString",
            })?,
            Symbol::SlopelineContour(point, _) => Ok(point),
            Symbol::BasemapContour(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "LineString",
            })?,
            Symbol::IndexContour(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "LineString",
            })?,
            Symbol::Formline(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "LineString",
            })?,
            Symbol::SlopelineFormline(point, _) => Ok(point),
            Symbol::SmallBoulder(point) => Ok(point),
            Symbol::LargeBoulder(point) => Ok(point),
            Symbol::GiganticBoulder(point) => Ok(point),
            Symbol::SandyGround(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "Polygon",
            })?,
            Symbol::BareRock(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "Polygon",
            })?,
            Symbol::RoughOpenLand(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "Polygon",
            })?,
            Symbol::LightGreen(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "Polygon",
            })?,
            Symbol::MediumGreen(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "Polygon",
            })?,
            Symbol::DarkGreen(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "Polygon",
            })?,
            Symbol::Building(_) => Err(Error::MismatchedGeometry {
                expected: "Point",
                found: "Polygon",
            })?,
        }
    }
}

impl TryFrom<Symbol> for LineString {
    type Error = crate::OmapError;

    fn try_from(value: Symbol) -> Result<LineString, Self::Error> {
        match value {
            Symbol::Contour(line_string) => Ok(line_string),
            Symbol::SlopelineContour(_, _) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Point",
            })?,
            Symbol::BasemapContour(line_string) => Ok(line_string),
            Symbol::IndexContour(line_string) => Ok(line_string),
            Symbol::Formline(line_string) => Ok(line_string),
            Symbol::SlopelineFormline(_, _) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Point",
            })?,
            Symbol::SmallBoulder(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Point",
            })?,
            Symbol::LargeBoulder(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Point",
            })?,
            Symbol::GiganticBoulder(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Point",
            })?,
            Symbol::SandyGround(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Polygon",
            })?,
            Symbol::BareRock(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Polygon",
            })?,
            Symbol::RoughOpenLand(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Polygon",
            })?,
            Symbol::LightGreen(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Polygon",
            })?,
            Symbol::MediumGreen(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Polygon",
            })?,
            Symbol::DarkGreen(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Polygon",
            })?,
            Symbol::Building(_) => Err(Error::MismatchedGeometry {
                expected: "LineString",
                found: "Polygon",
            })?,
        }
    }
}

impl TryFrom<Symbol> for Polygon {
    type Error = crate::OmapError;

    fn try_from(value: Symbol) -> Result<Polygon, Self::Error> {
        match value {
            Symbol::Contour(_) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "LineString",
            })?,
            Symbol::SlopelineContour(_, _) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "Point",
            })?,
            Symbol::BasemapContour(_) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "LineString",
            })?,
            Symbol::IndexContour(_) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "LineString",
            })?,
            Symbol::Formline(_) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "LineString",
            })?,
            Symbol::SlopelineFormline(_, _) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "Point",
            })?,
            Symbol::SmallBoulder(_) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "Point",
            })?,
            Symbol::LargeBoulder(_) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "Point",
            })?,
            Symbol::GiganticBoulder(_) => Err(Error::MismatchedGeometry {
                expected: "Polygon",
                found: "Point",
            })?,
            Symbol::SandyGround(polygon) => Ok(polygon),
            Symbol::BareRock(polygon) => Ok(polygon),
            Symbol::RoughOpenLand(polygon) => Ok(polygon),
            Symbol::LightGreen(polygon) => Ok(polygon),
            Symbol::MediumGreen(polygon) => Ok(polygon),
            Symbol::DarkGreen(polygon) => Ok(polygon),
            Symbol::Building(polygon) => Ok(polygon),
        }
    }
}
