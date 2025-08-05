mod barrier;
mod colors;
mod format_info;
mod geo_ref;
mod map_parts;
mod objects;
mod omap_editor;
mod symbols;
mod transform;

pub use omap_editor::OmapEditor;
pub use transform::Transform;

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
}
