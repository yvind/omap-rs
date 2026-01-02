use std::fs::File;
use std::io::{BufWriter, Read, Write};

use quick_xml::Reader;
use quick_xml::events::Event;

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
pub struct OmapEditor {
    pub notes: String,
    pub geo_info: GeoRef,
    pub colors: ColorSet,
    pub symbols: SymbolSet,
    pub parts: MapParts,
    pub omap_version: OmapVersion,
    pub xml_version: XmlVersion,
    // These are kept, but not exposed
    templates_and_view: String,
}

impl OmapEditor {
    /// Create an OmapEditor given a path to an omap file
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let mut reader = Reader::from_file(path)?;

        // these must be parsed successfully
        let mut georef = None;
        let mut colors = None;
        let mut symbols = None;
        let mut parts = None;

        // these have sensible defaults and are not worth bailing over if parsing fails
        let mut xml_version = XmlVersion::default();
        let mut omap_version = OmapVersion::default();
        let mut notes = String::new();
        let mut templates_and_view = include_str!("./templates_and_view_default.txt").to_string();

        let mut xml_buf = Vec::new();
        loop {
            match reader.read_event_into(&mut xml_buf)? {
                Event::Decl(dec) => {
                    xml_version = XmlVersion::parse(dec)?;
                }
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"map" => omap_version = OmapVersion::parse(&bytes_start)?,
                    b"notes" => notes = notes::parse(&mut reader)?,
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
                        let mut tav = format!(
                            "<templates{}>",
                            std::str::from_utf8(bytes_start.attributes_raw())?
                        );
                        // read the rest of the file
                        let _ = reader.stream().read_to_string(&mut tav)?;

                        // ignore everything after view end-tag
                        if let Some(index) = tav.find("</view>") {
                            let _ = tav.split_off(index + 7);
                            templates_and_view = tav;
                        }
                    }
                    _ => (),
                },
                Event::Eof => break,
                _ => (),
            }
        }

        Ok(OmapEditor {
            notes,
            geo_info: georef.ok_or(Error::ParseOmapFileError("Georeferencing".to_string()))?,
            colors: colors.ok_or(Error::ParseOmapFileError("Colors".to_string()))?,
            symbols: symbols.ok_or(Error::ParseOmapFileError("Symbols".to_string()))?,
            parts: parts.ok_or(Error::ParseOmapFileError("Parts".to_string()))?,
            omap_version,
            xml_version,
            templates_and_view,
        })
    }

    pub fn write_to_file(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        self.xml_version.write(&mut writer)?;
        self.omap_version.write(&mut writer)?;

        notes::write(self.notes.as_str(), &mut writer)?;

        self.geo_info.write(&mut writer)?;
        self.colors.write(&mut writer)?;

        writer.write_all("\n<barrier version=\"6\" required=\"0.6.0\">".as_bytes())?;

        self.symbols.write(&mut writer)?;
        self.parts.write(&mut writer)?;

        writer.write_all(self.templates_and_view.as_bytes())?;
        writer.write_all("\n</barrier>\n</map>".as_bytes())?;

        Ok(())
    }
}
