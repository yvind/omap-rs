use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Write};

#[cfg(feature = "geo_ref")]
use crate::geo_referencing::CrsType;
#[cfg(feature = "geo_ref")]
use geo_types::Coord;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, Event},
};

use crate::{
    colors::ColorSet,
    format_info::{OmapVersion, XmlDeclaration},
    geo_referencing::{AffineMapTransform, GeoRef, MapTransform},
    notes,
    parts::MapPart,
    parts::MapParts,
    symbols::SymbolSet,
    templates::Templates,
    view::View,
    {Error, Result},
};

const DEFAULT_ISOM_15000: &[u8] = include_bytes!("default_maps/isom_15000.omap");
const DEFAULT_ISOM_10000: &[u8] = include_bytes!("default_maps/isom_10000.omap");
const DEFAULT_ISSPROM_4000: &[u8] = include_bytes!("default_maps/issprom_4000.omap");

/// All objects are in map coordinates i.e given in mm of paper
/// relative the ref point with positive y towards the magnetic north
///
/// The Undo/Redo history and printer information is ignored
#[derive(Debug, Clone)]
pub struct Omap {
    /// Free-text notes embedded in the file.
    pub notes: String,
    /// Georeferencing information (scale, CRS, reference points).
    pub geo_referencing: GeoRef,
    /// The ordered set of colors used by symbols.
    pub colors: ColorSet,
    /// The set of map symbols.
    pub symbols: SymbolSet,
    /// The map parts (layers) containing objects.
    pub parts: MapParts,
    /// Background templates attached to the map.
    pub templates: Templates,
    /// View settings (zoom, grid, visibility).
    pub view: View,
}

impl Omap {
    /// Create a new georeferenced 1:15_000 map with a complete ISOM symbolset and color order
    #[cfg(feature = "geo_ref")]
    pub fn default_15_000_geo_referenced(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, 15_000)?;
        let mut omap = Self::from_bytes(DEFAULT_ISOM_15000)?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new georeferenced 1:10_000 map with a complete ISOM symbolset and color order
    #[cfg(feature = "geo_ref")]
    pub fn default_10_000_geo_referenced(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, 10_000)?;
        let mut omap = Self::from_bytes(DEFAULT_ISOM_10000)?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new georeferenced 1:4_000 map with a complete ISSprOM symbolset and color order
    #[cfg(feature = "geo_ref")]
    pub fn default_4_000_geo_referenced(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, 4_000)?;
        let mut omap = Self::from_bytes(DEFAULT_ISSPROM_4000)?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new 1:15_000 map with a complete ISOM symbolset and color order
    pub fn default_15_000() -> Result<Self> {
        Self::from_bytes(DEFAULT_ISOM_15000)
    }

    /// Create a new 1:10_000 map with a complete ISOM symbolset and color order
    pub fn default_10_000() -> Result<Self> {
        Self::from_bytes(DEFAULT_ISOM_10000)
    }

    /// Create a new 1:4_000 map with a complete ISSprOM symbolset and color order
    pub fn default_4_000() -> Result<Self> {
        Self::from_bytes(DEFAULT_ISSPROM_4000)
    }

    /// Create a new empty map
    pub fn new(scale_denominator: u32) -> Self {
        Omap {
            notes: Default::default(),
            geo_referencing: GeoRef::new(scale_denominator),
            colors: ColorSet(Vec::new()),
            symbols: SymbolSet {
                symbols: Vec::new(),
                name: "Custom".to_string(),
            },
            parts: MapParts(vec![MapPart::new("Map")]),
            templates: Default::default(),
            view: Default::default(),
        }
    }

    fn from_bytes(bytes: &'static [u8]) -> Result<Self> {
        Self::from_reader(Cursor::new(bytes))
    }

    fn from_reader<R: BufRead>(reader: R) -> Result<Self> {
        let mut reader = Reader::from_reader(reader);
        reader.config_mut().expand_empty_elements = true;

        // these must be parsed successfully
        let mut georef = None;
        let mut colors = None;
        let mut symbols = None;
        let mut parts = None;

        // these have sensible defaults and are not worth bailing over if parsing fails
        let mut notes = String::new();
        let mut templates = Templates::default();
        let mut view = View::default();

        let mut xml_buf = Vec::new();
        loop {
            match reader.read_event_into(&mut xml_buf)? {
                Event::Decl(dec) => XmlDeclaration::parse(dec)?,
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"map" => OmapVersion::parse(&bytes_start)?,
                    b"notes" => notes = notes::parse(&mut reader).unwrap_or_default(),
                    b"georeferencing" => georef = Some(GeoRef::parse(&mut reader, &bytes_start)?),
                    b"colors" => colors = Some(ColorSet::parse(&mut reader, &bytes_start)?),
                    b"symbols" => {
                        if let Some(colors) = &colors {
                            symbols = Some(SymbolSet::parse(&mut reader, &bytes_start, colors)?);
                        } else {
                            return Err(Error::ParseOmapFileError(
                                "Encountered Symbols before Colors".to_string(),
                            ));
                        }
                    }
                    b"parts" => {
                        if let Some(symbols) = &symbols {
                            parts = Some(MapParts::parse(&mut reader, symbols)?);
                        } else {
                            return Err(Error::ParseOmapFileError(
                                "Encountered Map parts before Symbols".to_string(),
                            ));
                        }
                    }
                    b"templates" => {
                        templates = Templates::parse(&mut reader, &bytes_start).unwrap_or_default()
                    }
                    b"view" => {
                        view = View::parse(&mut reader, &bytes_start, &mut templates)
                            .unwrap_or_default()
                    }
                    _ => (),
                },
                Event::Eof => break,
                _ => (),
            }
        }

        Ok(Omap {
            notes,
            geo_referencing: georef
                .ok_or(Error::ParseOmapFileError("Georeferencing".to_string()))?,
            colors: colors.ok_or(Error::ParseOmapFileError("Colors".to_string()))?,
            symbols: symbols.ok_or(Error::ParseOmapFileError("Symbols".to_string()))?,
            parts: parts.ok_or(Error::ParseOmapFileError("Parts".to_string()))?,
            templates,
            view,
        })
    }

    /// Create an [`Omap`] from a path to an `.omap` file.
    ///
    /// Parsing is intentionally permissive for some sections.
    /// This function falls back to sensible defaults
    /// for `notes`, `templates`, or `view`
    /// if those sections cannot be parsed
    ///
    /// `barrier`s, `undo` and `redo` sections of the file are ignored
    ///
    /// The core sections `georeferencing`, `colors`, `symbols`, and `parts`
    /// must still parse successfully or else loading fails.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = File::open(path)?;
        Self::from_reader(BufReader::new(file))
    }

    /// Write the map to an `.omap` file at the given path.
    pub fn write_to_file(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = Writer::new(BufWriter::new(file));

        XmlDeclaration::write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        OmapVersion::write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;

        notes::write(self.notes.as_str(), &mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;

        self.geo_referencing.write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        // write objects to a buffer
        let mut object_writer = Writer::new(Vec::new());
        self.parts.write(&mut object_writer, &self.symbols)?;
        let written_objects = object_writer.into_inner();

        // write symbolset to a buffer
        let mut symbol_writer = Writer::new(Vec::new());
        self.symbols.write(&mut symbol_writer, &self.colors)?;
        let written_symbols = symbol_writer.into_inner();

        // write colors
        self.colors.write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        writer.get_mut().flush()?;
        writer.get_mut().write_all(&written_symbols)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        writer.get_mut().flush()?;
        writer.get_mut().write_all(&written_objects)?;
        writer.get_mut().write_all(b"\n".as_slice())?;

        let vis = self.templates.write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        self.view.write(&mut writer, vis)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        writer.write_event(Event::End(BytesEnd::new("map")))?;

        Ok(())
    }

    /// Apply an [`AffineMapTransform`] to every object and non-georeferenced
    /// template in the map.
    ///
    /// Use this after changing the georeferencing (within the same projection)
    /// to keep objects and non-georeferenced templates at the same real-world
    /// positions. Obtain the transform with
    /// [`MapTransform::affine_between`].
    pub fn apply_affine(&mut self, transform: &AffineMapTransform) {
        for part in self.parts.0.iter_mut() {
            for object in part.iter_all_objects_mut() {
                object.apply_affine(transform);
            }
        }
        self.templates.apply_affine(transform);
    }

    /// Compute the affine transform between two [`MapTransform`]s and apply it
    /// to every object and non-georeferenced template. This is a convenience
    /// wrapper around [`MapTransform::affine_between`] + [`Omap::apply_affine`].
    pub fn apply_affine_between(&mut self, old: &MapTransform, new: &MapTransform) -> Result<()> {
        let affine = MapTransform::affine_between(old, new)?;
        self.apply_affine(&affine);
        Ok(())
    }
}
