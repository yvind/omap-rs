use std::fs::File;
use std::io::{BufWriter, Read, Write};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::editor::Transform;

use super::colors::ColorSet;
use super::format_info::{OmapVersion, XmlVersion};
use super::geo_ref::GeoRef;
use super::map_parts::MapParts;
use super::notes;
use super::symbols::SymbolSet;

use super::{Error, Result};

#[derive(Debug, Clone)]
pub struct OmapEditor {
    pub notes: String,
    pub geo_info: GeoRef,
    pub colors: ColorSet,
    pub symbols: SymbolSet,
    pub parts: MapParts,
    pub omap_version: OmapVersion,
    pub xml_version: XmlVersion,
    // These fields are not exposed
    barrier: String,
    templates_and_view: String,
}

impl OmapEditor {
    /// Create an OmapEditor given a path to an omap file
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let mut reader = Reader::from_file(path)?;

        let mut xml_version = None;
        let mut omap_version = None;
        let mut notes = None;
        let mut georef = None;
        let mut colors = None;
        let mut symbols = None;
        let mut parts = None;
        let mut templates_and_view = None;
        let mut barrier = None;

        let mut xml_buf = Vec::new();
        loop {
            match reader.read_event_into(&mut xml_buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"map" => omap_version = Some(OmapVersion::parse(&bytes_start)?),
                    b"notes" => notes = Some(notes::parse(&mut reader)?),
                    b"georeferencing" => georef = Some(GeoRef::parse(&mut reader, &bytes_start)?),
                    b"colors" => colors = Some(ColorSet::parse(&mut reader, &bytes_start)?),
                    b"barrier" => {
                        barrier = Some(format!(
                            "<barrier{}>",
                            std::str::from_utf8(bytes_start.attributes_raw())?
                        ));
                    }
                    b"symbols" => symbols = Some(SymbolSet::parse(&mut reader, &bytes_start)?),
                    b"parts" => parts = Some(MapParts::parse(&mut reader, &bytes_start)?),
                    b"templates" => {
                        let mut tav = format!(
                            "<templates{}>",
                            std::str::from_utf8(bytes_start.attributes_raw())?
                        );
                        reader.stream().read_to_string(&mut tav)?;

                        if let Some(index) = tav.find("</view>") {
                            let _ = tav.split_off(index + 7);
                        }

                        templates_and_view = Some(tav);
                    }
                    _ => (),
                },
                Event::Decl(dec) => {
                    xml_version = Some(XmlVersion::parse(dec)?);
                }
                Event::Eof => break,
                _ => (),
            }
        }

        Ok(OmapEditor {
            notes: notes.unwrap_or_default(),
            geo_info: georef.ok_or(Error::ParseOmapFileError("Georeferencing".to_string()))?,
            colors: colors.ok_or(Error::ParseOmapFileError("Colors".to_string()))?,
            symbols: symbols.ok_or(Error::ParseOmapFileError("Symbols".to_string()))?,
            parts: parts.ok_or(Error::ParseOmapFileError("Parts".to_string()))?,
            barrier: barrier.unwrap_or_default(),
            omap_version: omap_version.unwrap_or_default(),
            xml_version: xml_version.unwrap_or_default(),
            templates_and_view: templates_and_view.unwrap_or_default(),
        })
    }

    pub fn write_to_file(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let transform = Transform::new(&self.geo_info);

        self.xml_version.write(&mut writer)?;
        self.omap_version.write(&mut writer)?;
        writer.write_all(
            format!(
                "<notes>{}</notes>\n",
                quick_xml::escape::escape(self.notes.as_str())
            )
            .escape_default()
            .to_string()
            .as_bytes(),
        )?;

        self.geo_info.write(&mut writer)?;
        self.colors.write(&mut writer)?;

        writer.write_all(self.barrier.as_bytes())?;

        self.symbols.write(&mut writer)?;
        self.parts.write(&mut writer, &transform)?;

        writer.write_all(self.templates_and_view.as_bytes())?;
        writer.write_all("</barrier>\n</map>".as_bytes())?;

        Ok(())
    }
}
