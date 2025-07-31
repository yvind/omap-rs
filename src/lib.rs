//! Write Open Orienteering Mapper's .omap files in Rust
//!
//! # Example
//!
//! ```
//! use omap::{
//!     objects::{AreaObject, LineObject, PointObject, TextObject, TagTrait},
//!     symbols::{AreaSymbol, LineSymbol, PointSymbol, TextSymbol},
//!     Omap, Scale,
//!     };
//! use geo_types::{Coord, LineString, Polygon, Point};
//! use std::{path::PathBuf, str::FromStr};
//!
//! let map_center = Coord {x: 463_575.5, y: 6_833_849.6};
//! let map_center_elevation_meters = 2_469.;
//! let crs_epsg_code = 25832;
//!
//! let mut omap = Omap::new(
//!     map_center,
//!     Scale::S15_000,
//!     Some(crs_epsg_code),
//!     Some(map_center_elevation_meters)
//! ).expect("Could not make map with the given CRS-code");
//!
//! // coordinates of geometry are in the same units as the map_center, but relative the map_center
//! let polygon = Polygon::new(
//!     LineString::new(vec![
//!         Coord {x: -50., y: -50.},
//!         Coord {x: -50., y: 50.},
//!         Coord {x: 50., y: 50.},
//!         Coord {x: 50., y: -50.},
//!         Coord {x: -50., y: -50.},
//!     ]), vec![]);
//! let mut area_object = AreaObject::from_polygon(polygon, AreaSymbol::RoughVineyard, 45.0_f64.to_radians());
//! area_object.add_tag("tag_key", "tag_value");
//!
//! let line_string = LineString::new(
//!         vec![
//!             Coord {x: -60., y: 20.},
//!             Coord {x: -20., y: 25.},
//!             Coord {x: 0., y: 27.5},
//!             Coord {x: 20., y: 26.},
//!             Coord {x: 40., y: 22.5},
//!             Coord {x: 60., y: 20.},
//!             Coord {x: 60., y: -20.},
//!             Coord {x: -60., y: -20.},
//!         ]
//!     );
//! let mut line_object = LineObject::from_line_string(line_string, LineSymbol::Contour);
//! line_object.add_elevation_tag(20.);
//!
//! let point = Point::new(0.0_f64, 0.0_f64);
//! let point_object = PointObject::from_point(point, PointSymbol::ElongatedDotKnoll, -45.0_f64.to_radians());
//!
//! let text_point = Point::new(0.0_f64, -30.0_f64);
//! let text = "some text".to_string();
//! let text_object = TextObject::from_point(text_point, TextSymbol::SpotHeight, text);
//!
//! omap.add_object(area_object);
//! omap.add_object(line_object);
//! omap.add_object(point_object);
//! omap.add_object(text_object);
//!
//! let max_bezier_deviation_meters = 2.5;
//!
//! omap.write_to_file(
//!     PathBuf::from_str("./my_map.omap").unwrap(),
//!     Some(max_bezier_deviation_meters)
//! ).expect("Could not write to file");
//! ```

#![deny(
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    missing_debug_implementations,
    missing_docs,
    non_ascii_idents,
    noop_method_call,
    rust_2021_incompatible_closure_captures,
    rust_2021_incompatible_or_patterns,
    rust_2021_prefixes_incompatible_syntax,
    rust_2021_prelude_collisions,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    warnings
)]

/// Objects module
pub mod objects;
mod omap;
mod scale;
mod serialize;
/// Symbols module
pub mod symbols;
mod transform;

pub use self::omap::Omap;
pub use self::scale::Scale;

/// crate result
pub type OmapResult<T> = Result<T, OmapError>;

use thiserror::Error;
/// crate error
#[derive(Error, Debug)]
pub enum OmapError {
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
