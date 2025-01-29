pub mod area_object;
pub mod line_object;
mod map_geo_traits;
mod map_object;
pub mod omap;
pub mod point_object;
pub mod symbol;
pub mod tag;

pub use self::area_object::AreaObject;
pub use self::line_object::LineObject;
pub use self::map_object::MapObject;
pub use self::omap::Omap;
pub use self::point_object::PointObject;
pub use self::symbol::Symbol;
pub use self::tag::Tag;

#[derive(Clone, Copy)]
pub enum Scale {
    S7_500,
    S10_000,
    S15_000,
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
}
