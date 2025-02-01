mod area_object;
mod line_object;
mod map_coord;
mod map_object;
mod omap;
mod point_object;
mod symbol;
mod tag;

pub use self::area_object::AreaObject;
pub use self::line_object::LineObject;
pub use self::map_object::{MapObject, TagTrait};
pub use self::omap::Omap;
pub use self::point_object::PointObject;
pub use self::symbol::*;
pub use self::tag::Tag;

#[derive(Clone, Copy)]
pub enum Scale {
    S10_000,
    S15_000,
}

use std::fmt;
impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scale::S10_000 => write!(f, "10000"),
            Scale::S15_000 => write!(f, "15000"),
        }
    }
}

pub type OmapResult<T> = std::result::Result<T, OmapError>;

use thiserror::Error;
#[derive(Error, Debug)]
pub enum OmapError {
    #[error("Map coordinate overflow, double check that all lidar files are over the same general area and in the same coordinate refrence system.")]
    MapCoordinateOverflow,
    #[error(transparent)]
    MismatchedGeometry(#[from] geo_types::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Proj(#[from] proj4rs::errors::Error),
    #[error(transparent)]
    GeoMagnetic(#[from] world_magnetic_model::Error),
}
