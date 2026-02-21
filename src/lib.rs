#![deny(
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    missing_debug_implementations,
    //missing_docs,
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
    //unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    warnings
)]

pub mod colors;
pub mod format_info;
pub mod geo_referencing;
mod notes;
pub mod objects;
pub mod omap;
pub mod parts;
pub mod symbols;
pub mod templates;
pub mod view;

use std::{io::BufWriter, str::FromStr};

pub use omap::Omap;

type Result<T> = std::result::Result<T, Error>;

use quick_xml::events::BytesStart;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    IntoInnerError(#[from] std::io::IntoInnerError<BufWriter<Vec<u8>>>),
    #[error("format error")]
    InvalidFormat(String),
    #[error("format coord error")]
    InvalidCoordinate(String),
    #[error(transparent)]
    AttrError(#[from] quick_xml::events::attributes::AttrError),
    #[error(transparent)]
    StrUtf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    StringUtf8Error(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    EncodingError(#[from] quick_xml::encoding::EncodingError),
    #[error(transparent)]
    EscapeError(#[from] quick_xml::escape::EscapeError),
    #[error("Could not merge map parts. Check that the indices are different and in range")]
    MapPartMergeError,
    #[error("XML-encoding {0} is not supported")]
    UnsupportedEncoding(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("Part {0} of file could not parsed")]
    ParseOmapFileError(String),
    #[error(transparent)]
    BorrowError(#[from] std::cell::BorrowError),
    #[error(transparent)]
    BorrowMutError(#[from] std::cell::BorrowMutError),
    #[error(transparent)]
    BezierConversionError(#[from] linestring2bezier::Error),
    #[error("Color definition error")]
    ColorError,
    #[error("Symbol definition error")]
    SymbolError,
    #[error("Template error")]
    TemplateError,
    #[error("View error")]
    ViewError,
    #[error("Object error")]
    ObjectError,
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    #[error("A provided map coordinate is outside the range for writing")]
    TransfromError,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Code {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl FromStr for Code {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self> {
        let mut parts = value.split('.').take(3);
        Ok(Code {
            major: parts.next().ok_or(Error::SymbolError)?.parse()?,
            minor: parts.next().and_then(|i| i.parse().ok()).unwrap_or(0),
            patch: parts.next().and_then(|i| i.parse().ok()).unwrap_or(0),
        })
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// parse helpers
pub(crate) fn parse_attr<T: FromStr>(value: std::borrow::Cow<'_, [u8]>) -> Option<T> {
    std::str::from_utf8(value.as_ref())
        .ok()
        .and_then(|s| T::from_str(s).ok())
}

pub(crate) fn try_get_attr<T: FromStr>(bytes: &BytesStart<'_>, attr: &str) -> Option<T> {
    bytes
        .try_get_attribute(attr)
        .ok()
        .flatten()
        .and_then(|a| parse_attr(a.value))
}
