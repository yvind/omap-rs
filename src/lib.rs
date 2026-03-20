//! A Rust library for reading and writing OpenOrienteering Mapper (`.omap`) files.
//!
//! All map coordinates are given in millimetres on paper, relative to the
//! reference point, with the positive y-axis pointing towards magnetic north.

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
    rust_2024_prelude_collisions,
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

/// Color definitions: spot colors, mixed colors, CMYK, RGB.
pub mod colors;
/// File-format version information (XML and OMAP versions).
pub mod format_info;
/// Coordinate-reference-system and projection helpers.
pub mod geo_referencing;
mod notes;
/// Map objects: points, lines, areas, and text.
pub mod objects;
/// The top-level OMAP document type.
pub mod omap;
/// Map parts (layers) and their contained objects.
pub mod parts;
/// Symbol definitions: point, line, area, text, and combined symbols.
pub mod symbols;
/// Background-template support (images, tracks, GDAL/OGR layers).
pub mod templates;
mod utils;
/// View settings: zoom, grid, template visibility.
pub mod view;

use std::io::BufWriter;

pub use omap::Omap;
pub use utils::{Code, NonNegativeF64, UnitF64};

type Result<T> = std::result::Result<T, Error>;

use thiserror::Error;

/// Errors that can occur when reading, writing, or manipulating OMAP data.
#[derive(Debug, Error)]
pub enum Error {
    /// An error from the XML parser.
    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),
    /// An I/O error.
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    /// An error flushing an internal `BufWriter`.
    #[error(transparent)]
    IntoInnerError(#[from] std::io::IntoInnerError<BufWriter<Vec<u8>>>),
    /// A generic file-format error.
    #[error("format error")]
    InvalidFormat(String),
    /// A coordinate could not be parsed.
    #[error("format coord error")]
    InvalidCoordinate(String),
    /// An XML attribute error.
    #[error(transparent)]
    AttrError(#[from] quick_xml::events::attributes::AttrError),
    /// A `&str` UTF-8 conversion error.
    #[error(transparent)]
    StrUtf8Error(#[from] std::str::Utf8Error),
    /// A `String` UTF-8 conversion error.
    #[error(transparent)]
    StringUtf8Error(#[from] std::string::FromUtf8Error),
    /// An XML encoding error.
    #[error(transparent)]
    EncodingError(#[from] quick_xml::encoding::EncodingError),
    /// An XML escape-sequence error.
    #[error(transparent)]
    EscapeError(#[from] quick_xml::escape::EscapeError),
    /// Map parts could not be merged (indices out of range or identical).
    #[error("Could not merge map parts. Check that the indices are different and in range")]
    MapPartMergeError,
    /// The XML encoding is not supported.
    #[error("XML-encoding {0} is not supported")]
    UnsupportedEncoding(String),
    /// Failed to parse an integer.
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    /// Failed to parse a float.
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    /// A section of the `.omap` file could not be parsed.
    #[error("Part {0} of file could not parsed")]
    ParseOmapFileError(String),
    /// A `RefCell` borrow failed.
    #[error(transparent)]
    BorrowError(#[from] std::cell::BorrowError),
    /// A `RefCell` mutable borrow failed.
    #[error(transparent)]
    BorrowMutError(#[from] std::cell::BorrowMutError),
    /// A Bézier-curve conversion error.
    #[error(transparent)]
    BezierConversionError(#[from] linestring2bezier::Error),
    /// An invalid color definition.
    #[error("Color definition error")]
    ColorError,
    /// An invalid symbol definition.
    #[error("Symbol definition error")]
    SymbolError(String),
    /// A template-related error.
    #[error("Template error")]
    TemplateError,
    /// A view-related error.
    #[error("View error")]
    ViewError,
    /// An object-related error.
    #[error("Object error")]
    ObjectError,
    /// The value is not in the unit interval `[0, 1]`.
    #[error("The value is not in the unit interval and cannot be converted to a UnitF64")]
    NotInUnitInterval,
    /// The value is negative.
    #[error("The value is not non-negative and cannot be converted to a NonNegativeF64")]
    NotNonNegativeF64,
    /// Infallible conversion (required by `From` blanket impl).
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    /// A map coordinate exceeds the file-format range.
    #[error("A provided map coordinate is outside the range for writing")]
    MapCoordOutOfBounds,
    /// An error from the World Magnetic Model.
    #[cfg(feature = "geo_ref")]
    #[error(transparent)]
    WmmError(#[from] world_magnetic_model::Error),
    /// A proj4 projection error.
    #[cfg(feature = "geo_ref")]
    #[error(transparent)]
    ProjError(#[from] proj4rs::errors::Error),
    /// An Error when parsing a `Code` from an empty string
    #[error("Tried to parse a Code from an empty string")]
    EmptyCode,
}
