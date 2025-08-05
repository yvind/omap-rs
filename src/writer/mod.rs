mod bezier_error;
/// Objects module
pub mod objects;
mod omap_writer;
mod scale;
mod serialize;
/// Symbols module
pub mod symbols;
mod transform;

pub use bezier_error::BezierError;
pub use omap_writer::OmapWriter;
pub use scale::Scale;

/// writer result
pub type Result<T> = std::result::Result<T, Error>;

use thiserror::Error;
/// writer error
#[derive(Error, Debug)]
pub enum Error {
    /// Map coordinate overflow
    #[error("Map coordinate overflow")]
    MapCoordinateOverflow,
    /// Wrong geo_types geometry for a symbol
    #[error(transparent)]
    MismatchedGeometry(#[from] geo_types::Error),
    /// IO error
    #[error(transparent)]
    IO(#[from] std::io::Error),
    /// Projection error
    #[cfg(feature = "geo_ref")]
    #[error(transparent)]
    Proj(#[from] proj4rs::errors::Error),
    /// World magnetic model declination error
    #[cfg(feature = "geo_ref")]
    #[error(transparent)]
    GeoMagnetic(#[from] world_magnetic_model::Error),
    /// The geo-referencing feature is de-activated, but an EPSG code was passed to new
    #[error("The geo-referencing feature is de-activated (activated by default)")]
    DisabledGeoReferencingFeature,
    /// The symbol type and the object type do not match
    #[error("Wrong Symbol type for object")]
    MismatchingSymbolAndObject,
}
