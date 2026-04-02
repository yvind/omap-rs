use std::fs::File;
use std::io::{BufWriter, Write};

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
    format_info::Barrier,
    format_info::{OmapVersion, XmlVersion},
    geo_referencing::GeoRef,
    notes,
    parts::MapPart,
    parts::MapParts,
    symbols::SymbolSet,
    templates::Templates,
    view::View,
    {Error, Result},
};

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
    /// The OMAP file-format version.
    pub omap_version: OmapVersion,
    /// The XML declaration version and encoding.
    pub xml_version: XmlVersion,
    /// Background templates attached to the map.
    pub templates: Templates,
    /// View settings (zoom, grid, visibility).
    pub view: View,
    symbol_barrier: Barrier,
}

impl Omap {
    /// Create a new georeferenced 1:15_000 map with a complete ISOM symbolset and color order
    #[cfg(feature = "geo_ref")]
    pub fn default_15_000(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, 15_000)?;
        let mut omap = Self::from_path("./src/default_maps/isom_15000.omap")?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new georeferenced 1:10_000 map with a complete ISOM symbolset and color order
    #[cfg(feature = "geo_ref")]
    pub fn default_10_000(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, 10_000)?;
        let mut omap = Self::from_path("./src/default_maps/isom_10000.omap")?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new georeferenced 1:4_000 map with a complete ISSprOM symbolset and color order
    #[cfg(feature = "geo_ref")]
    pub fn default_4_000(
        projected_ref_point: Coord,
        crs: CrsType,
        meters_above_sea: f64,
    ) -> Result<Self> {
        let geo_ref = GeoRef::initialize(projected_ref_point, crs, meters_above_sea, 4_000)?;
        let mut omap = Self::from_path("./src/default_maps/issprom_4000.omap")?;
        omap.geo_referencing = geo_ref;
        Ok(omap)
    }

    /// Create a new 1:15_000 map with a complete ISOM symbolset and color order
    #[cfg(not(feature = "geo_ref"))]
    pub fn default_15_000() -> Result<Self> {
        Self::from_path("./src/default_maps/isom_15000.omap")
    }

    /// Create a new 1:10_000 map with a complete ISOM symbolset and color order
    #[cfg(not(feature = "geo_ref"))]
    pub fn default_10_000() -> Result<Self> {
        Self::from_path("./src/default_maps/isom_10000.omap")
    }

    /// Create a new 1:4_000 map with a complete ISSprOM symbolset and color order
    #[cfg(not(feature = "geo_ref"))]
    pub fn default_4_000() -> Result<Self> {
        Self::from_path("./src/default_maps/issprom_4000.omap")
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
            omap_version: Default::default(),
            xml_version: Default::default(),
            templates: Default::default(),
            view: Default::default(),
            symbol_barrier: Default::default(),
        }
    }

    /// Create an Omap given a path to an omap file
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let mut reader = Reader::from_file(path)?;
        reader.config_mut().expand_empty_elements = true;

        // these must be parsed successfully
        let mut georef = None;
        let mut colors = None;
        let mut symbols = None;
        let mut parts = None;

        // these have sensible defaults and are not worth bailing over if parsing fails
        let mut symbol_barrier = Barrier::default();
        let mut xml_version = XmlVersion::default();
        let mut omap_version = OmapVersion::default();
        let mut notes = String::new();
        let mut templates = Templates::default();
        let mut view = View::default();

        let mut xml_buf = Vec::new();
        loop {
            match reader.read_event_into(&mut xml_buf)? {
                Event::Decl(dec) => {
                    xml_version = XmlVersion::parse(dec)?;
                }
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"map" => omap_version = OmapVersion::parse(&bytes_start).unwrap_or_default(),
                    b"barrier" => symbol_barrier = Barrier::parse(&bytes_start).unwrap_or_default(),
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
            symbol_barrier,
            omap_version,
            xml_version,
            templates,
            view,
        })
    }

    /// Write the map to an `.omap` file at the given path.
    pub fn write_to_file(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = Writer::new(BufWriter::new(file));

        self.xml_version.write(&mut writer)?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        self.omap_version.write(&mut writer)?;
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
        self.symbol_barrier.write(&mut writer)?;
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

        writer.write_event(Event::End(BytesEnd::new("barrier")))?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        writer.write_event(Event::End(BytesEnd::new("map")))?;

        Ok(())
    }
}
