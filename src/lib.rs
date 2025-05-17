//! Write Open Orienteering Mapper's .omap files in Rust
//!
//! # Example
//!
//! ```
//! use omap::{Omap, Scale, AreaObject, AreaSymbol, LineObject, LineSymbol, TagTrait, PointSymbol, PointObject, TextSymbol, TextObject};
//! use geo_types::{Coord, LineString, Polygon, Point};
//! use std::{path::PathBuf, str::FromStr};
//!
//! let map_center = Coord {x: 463_575.5, y: 6_833_849.6};
//!
//! let mut omap = Omap::new(map_center, Scale::S15_000, Some(25832), Some(2_469.)).expect("Could not make map with the given CRS-code");
//!
//! // coordinates of geometry is relative the map_center
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
//!             Coord {x: 20., y: 25.},
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
//! omap.add_object(area_object.into());
//! omap.add_object(line_object.into());
//! omap.add_object(point_object.into());
//! omap.add_object(text_object.into());
//!
//! let max_bezier_error = 5.;
//!
//! omap.write_to_file(PathBuf::from_str("./my_map.omap").unwrap(), Some(max_bezier_error)).expect("Could not write to file");
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

mod area_object;
mod geometry;
mod line_object;
mod map_object;
mod omap;
mod point_object;
mod scale;
mod symbol;
mod text_object;

pub use self::area_object::AreaObject;
pub use self::line_object::LineObject;
pub use self::map_object::{MapObject, TagTrait};
pub use self::omap::Omap;
pub use self::point_object::PointObject;
pub use self::scale::Scale;
pub use self::symbol::*;
pub use self::text_object::TextObject;

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
    #[error(transparent)]
    Proj(#[from] proj4rs::errors::Error),
    /// World magnetic model declination error
    #[error(transparent)]
    GeoMagnetic(#[from] world_magnetic_model::Error),
}
