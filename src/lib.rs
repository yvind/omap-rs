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

/// Color definitions: color set, spot colors, mixed colors, CMYK, RGB.
pub mod colors;
mod format_info;
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

use std::{fmt::Debug, io::BufWriter};

pub use omap::Omap;
pub use utils::{Code, NonNegativeF64, UnitF64};

type Result<T> = std::result::Result<T, Error>;

use thiserror::Error;

/// A high-level section or parser context in an `.omap` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmapSection {
    /// The XML declaration.
    XmlDeclaration,
    /// The root `map` element.
    Map,
    /// The `georeferencing` section.
    Georeferencing,
    /// The `colors` section.
    Colors,
    /// The `symbols` section.
    Symbols,
    /// The `parts` section.
    Parts,
    /// A `tags` section.
    Tags,
    /// A point object.
    PointObject,
    /// A line object.
    LineObject,
    /// An area object.
    AreaObject,
    /// A text object.
    TextObject,
    /// A point symbol.
    PointSymbol,
    /// A line symbol.
    LineSymbol,
    /// An area symbol.
    AreaSymbol,
    /// A text symbol.
    TextSymbol,
    /// A combined area symbol.
    CombinedAreaSymbol,
    /// A point-symbol element.
    Element,
    /// A fill pattern.
    FillPattern,
    /// A fill-pattern point symbol.
    FillPatternPoint,
    /// A combined-symbol private part.
    PrivatePart,
    /// A skipped combined-symbol part.
    SkippedPart,
    /// A color set.
    ColorSet,
    /// A color definition.
    Color,
    /// Templates.
    Templates,
    /// A template.
    Template,
    /// Template transformations.
    TemplateTransformations,
    /// A template pass point.
    PassPoint,
    /// A transformation matrix.
    Matrix,
    /// A coordinate wrapper element.
    CoordinateWrapper,
    /// A map part.
    MapPart,
}

/// A coordinate component required while parsing geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateComponent {
    /// The x coordinate.
    X,
    /// The y coordinate.
    Y,
}

/// The kind of object being parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// A point object.
    Point,
    /// A text object.
    Text,
}

/// Errors that can occur when reading, writing, or manipulating OMAP data.
#[derive(Debug, Error)]
pub enum Error {
    /// An error when converting from str.
    #[error("An error when converting from str")]
    FromStrError,
    /// An error from the XML parser.
    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),
    /// An I/O error.
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    /// An error flushing an internal `BufWriter`.
    #[error(transparent)]
    IntoInnerError(#[from] std::io::IntoInnerError<BufWriter<Vec<u8>>>),
    /// The root `map` element has an unsupported XML namespace.
    #[error("unsupported OMAP XML namespace")]
    UnsupportedOmapNamespace,
    /// The root `map` element is missing its version.
    #[error("missing OMAP version")]
    MissingOmapVersion,
    /// The root `map` element has an unsupported version.
    #[error("unsupported OMAP version {0}")]
    UnsupportedOmapVersion(u8),
    /// The XML declaration has an unsupported XML version.
    #[error("unsupported XML version")]
    UnsupportedXmlVersion,
    /// A coordinate component is missing.
    #[error("missing coordinate component {0:?}")]
    MissingCoordinateComponent(CoordinateComponent),
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
    /// The XML declaration is missing an encoding.
    #[error("missing XML encoding")]
    MissingXmlEncoding,
    /// The XML encoding is not supported by this crate.
    #[error("unsupported XML encoding")]
    UnsupportedXmlEncoding,
    /// Failed to parse an integer.
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    /// Failed to parse a float.
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    /// The parser reached EOF before a section ended.
    #[error("unexpected EOF while parsing {0:?}")]
    UnexpectedEof(OmapSection),
    /// A required top-level section was not found.
    #[error("missing required OMAP section {0:?}")]
    MissingRequiredSection(OmapSection),
    /// A section appeared before another required section.
    #[error("OMAP section {section:?} appeared before required section {required_before:?}")]
    SectionOutOfOrder {
        /// The section that was encountered.
        section: OmapSection,
        /// The section that must be parsed first.
        required_before: OmapSection,
    },
    /// The georeferencing section is missing the map scale.
    #[error("missing georeferencing map scale")]
    MissingMapScale,
    /// The georeferencing section could not be parsed.
    #[error("invalid georeferencing")]
    InvalidGeoreferencing,
    /// A map object is missing its type.
    #[error("missing object type")]
    MissingObjectType,
    /// A map object references an unknown symbol id.
    #[error("unknown object symbol id {0}")]
    UnknownObjectSymbolId(i32),
    /// A parsed object is missing its geometry.
    #[error("missing {0:?} object geometry")]
    MissingObjectGeometry(ObjectKind),
    /// A text object is missing coordinate data.
    #[error("missing text object coordinates")]
    MissingTextObjectCoordinates,
    /// A symbol is missing its id.
    #[error("missing symbol id")]
    MissingSymbolId,
    /// A symbol has an unknown type value.
    #[error("unknown symbol type {0}")]
    UnknownSymbolType(u8),
    /// A point-symbol element has an unknown symbol type value.
    #[error("unknown point-symbol element symbol type {0}")]
    UnknownElementSymbolType(u8),
    /// A point-symbol element has an unknown object type value.
    #[error("unknown point-symbol element object type {0}")]
    UnknownElementObjectType(u8),
    /// A point-symbol element has mismatched symbol and object data.
    #[error("point-symbol element symbol/object type mismatch")]
    ElementSymbolObjectMismatch,
    /// A point-symbol element object appeared before its symbol.
    #[error("point-symbol element object appeared before symbol")]
    ElementObjectBeforeSymbol,
    /// A point-symbol element is missing symbol or object data.
    #[error("point-symbol element is missing symbol or object data")]
    MissingElementData,
    /// A point-symbol element is missing a required point symbol.
    #[error("point-symbol element is missing a point symbol part")]
    MissingPointSymbolElementPart,
    /// A symbol id is outside the declared symbol count.
    #[error("symbol id {0} is outside the declared symbol count")]
    SymbolIdOutOfRange(usize),
    /// A symbol id appears more than once.
    #[error("duplicate symbol id {0}")]
    DuplicateSymbolId(usize),
    /// The declared symbol count does not match the parsed symbols.
    #[error("symbol count does not match parsed symbols")]
    SymbolCountMismatch,
    /// A non-combined symbol declares combined-symbol components.
    #[error("components found in non-combined symbol")]
    ComponentsInNonCombinedSymbol,
    /// A fill pattern has an unknown type value.
    #[error("unknown fill pattern type {0}")]
    UnknownFillPatternType(u8),
    /// A point fill pattern is missing its point symbol.
    #[error("missing point symbol in point fill pattern")]
    MissingPointPatternSymbol,
    /// A private combined-symbol part has an unknown symbol type.
    #[error("unknown private part symbol type {0}")]
    UnknownPrivatePartSymbolType(u8),
    /// A private combined-symbol part is empty.
    #[error("empty private combined-symbol part")]
    EmptyPrivatePart,
    /// A color definition is missing its id.
    #[error("missing color id")]
    MissingColorId,
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
    /// A symbol definition would create a cycle.
    #[error("cyclic symbol definition")]
    CyclicSymbolDefinition,
    /// A symbol could not be borrowed while checking for cycles.
    #[error("cannot borrow symbol during cycle check")]
    SymbolCycleBorrow,
    /// A combined symbol references a symbol outside the symbol set.
    #[error("symbol set index {0} out of range")]
    SymbolSetIndexOutOfRange(usize),
    /// A combined area symbol references a point or text symbol.
    #[error("combined area symbol contains a point or text symbol")]
    CombinedSymbolContainsPointOrText,
    /// A combined line symbol references a non-line symbol.
    #[error("combined line symbol contains a non-line symbol")]
    CombinedLineSymbolContainsNonLine,
    /// A clipping option value is unknown.
    #[error("unknown clipping option")]
    UnknownClippingOption,
    /// A line cap style value is unknown.
    #[error("unknown line cap style")]
    UnknownCapStyle,
    /// A line join style value is unknown.
    #[error("unknown line join style")]
    UnknownJoinStyle,
    /// A mid-symbol placement value is unknown.
    #[error("unknown mid-symbol placement")]
    UnknownMidSymbolPlacement,
    /// A template-related error.
    #[error("Template error")]
    TemplateError,
    /// Could not convert signed integer to unsigned integer
    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),
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
    /// An Error when parsing a [Code] from an empty string
    #[error("Tried to parse a Code from an empty string")]
    EmptyCode,
    /// An error from the World Magnetic Model.
    #[cfg(feature = "geo_ref")]
    #[error(transparent)]
    WmmError(#[from] world_magnetic_model::Error),
    /// A proj4 projection error.
    #[cfg(feature = "geo_ref")]
    #[error(transparent)]
    ProjError(#[from] proj_core::Error),
    /// Could not parse the crs definition to a CrsDef object
    #[cfg(feature = "geo_ref")]
    #[error(transparent)]
    ProjParseError(#[from] proj_wkt::ParseError),
    /// A tolerance error when calculating the scale factor during geo referencing
    #[cfg(feature = "geo_ref")]
    #[error(
        "Encountered a tolerence error when calculating the scale factor during geo referencing"
    )]
    ProjScaleToleranceError,
    /// Affine transforms are only available between changed geo referencing within the same projection
    #[error(
        "Affine transforms are only available between changed geo referencing within the same projection"
    )]
    CannotGetAffineTransformBetweenDifferentProjections,
    /// Tried to call try into on non-compatible symbols
    #[error("Tried to call try into on non-compatible symbols")]
    SymbolConversionError,
}
