use std::fs::File;
use std::io::{BufWriter, Write};

use quick_xml::events::{BytesEnd, Event};
use quick_xml::{Reader, Writer};

use crate::format_info::Barrier;
use crate::parts::MapPart;
use crate::templates::Templates;
use crate::view::View;

use super::colors::ColorSet;
use super::format_info::{OmapVersion, XmlVersion};
use super::geo_referencing::GeoRef;
use super::notes;
use super::parts::MapParts;
use super::symbols::SymbolSet;
use super::{Error, Result};

/// All objects are in map coordinates i.e given in mm of paper
/// relative the ref point with positive y towards the magnetic north
///
/// The Undo history is wiped
///
/// To transform the coordinates to projected coordinates get the transform
/// from GeoRef::get_transform(&self) and pass it to the MapObject::to_proj_object(self, transform: &Transform) -> ProjObject
///
/// For converting the other way use the inverse functions:
/// GeoRef::get_inverse_transform(&self)
/// ProjObject::to_map_object(self, inv_transform: &Transform) -> MapObject
///
#[derive(Debug, Clone)]
pub struct Omap {
    pub notes: String,
    pub geo_info: GeoRef,
    pub colors: ColorSet,
    pub symbols: SymbolSet,
    pub parts: MapParts,
    pub omap_version: OmapVersion,
    pub xml_version: XmlVersion,
    pub templates: Templates,
    pub view: View,
    // These are kept, but not exposed
    symbol_barrier: Barrier,
}

impl Omap {
    /// Create a new empty map
    pub fn new(scale_denominator: u32) -> Self {
        Omap {
            notes: Default::default(),
            geo_info: GeoRef::new(scale_denominator),
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
            geo_info: georef.ok_or(Error::ParseOmapFileError("Georeferencing".to_string()))?,
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

    pub fn write_to_file(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = Writer::new(BufWriter::new(file));

        self.xml_version.write(&mut writer)?;
        self.omap_version.write(&mut writer)?;

        notes::write(self.notes.as_str(), &mut writer)?;

        self.geo_info.write(&mut writer)?;
        // write objects to a buffer
        let mut object_writer = Writer::new(Vec::new());
        self.parts.write(&mut object_writer)?;
        let written_objects = object_writer.into_inner();

        // write symbolset to a buffer
        let mut symbol_writer = Writer::new(Vec::new());
        self.symbols.write(&mut symbol_writer, &self.colors)?;
        let written_symbols = symbol_writer.into_inner();

        // write colors
        self.colors.write(&mut writer)?;
        self.symbol_barrier.write(&mut writer)?;
        writer.get_mut().flush()?;
        writer.get_mut().write_all(&written_symbols);
        writer.get_mut().flush()?;
        writer.get_mut().write_all(&written_objects);

        let vis = self.templates.write(&mut writer)?;
        self.view.write(&mut writer, vis)?;

        writer.write_event(Event::End(BytesEnd::new("barrier")))?;
        writer.write_event(Event::End(BytesEnd::new("map")))?;

        Ok(())
    }
}
