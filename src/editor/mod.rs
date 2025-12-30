mod colors;
mod format_info;
mod geo_ref;
mod notes;
mod objects;
mod omap_editor;
mod parts;
mod symbols;

pub use omap_editor::OmapEditor;

/// editor results
pub type Result<T> = std::result::Result<T, Error>;

use thiserror::Error;
/// editor errors
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
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
}
