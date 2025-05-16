//! Write Open Orienteering Mapper's .omap files in Rust

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
