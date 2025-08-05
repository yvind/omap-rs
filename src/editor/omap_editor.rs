use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::editor::Transform;

use super::barrier::Barrier;
use super::colors::ColorSet;
use super::format_info::{OmapVersion, XmlVersion};
use super::geo_ref::GeoRef;
use super::map_parts::MapParts;
use super::symbols::SymbolSet;

use super::{Error, Result};

#[derive(Debug, Clone)]
pub struct OmapEditor {
    pub notes: String,
    pub geo_info: GeoRef,
    pub colors: ColorSet,
    pub symbols: SymbolSet,
    pub parts: MapParts,
    pub format_version: OmapVersion,
    pub xml_version: XmlVersion,
    // These fields are not exposed
    barrier: Barrier,
    templates_and_view: String,
}

impl OmapEditor {
    /// Create an OmapEditor from a path to an omap file
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let mut reader = Reader::from_file(path)?;

        let mut xml_buf = Vec::new();
        loop {
            match reader.read_event_into(&mut xml_buf)? {
                Event::Start(bytes_start) => todo!(),
                Event::End(bytes_end) => todo!(),
                Event::Text(bytes_text) => todo!(),
                Event::Eof => break,
                _ => (),
            }
        }

        Ok(OmapEditor {
            notes: (),
            geo_info: (),
            colors: (),
            symbols: (),
            parts: (),
            barrier: (),
            header,
            templates_and_view: (),
        })
    }

    pub fn write_to_file(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let transform = Transform::new(&self.geo_info);

        writer.write_all(self.header.as_bytes())?;
        writer.write_all(format!("<notes>{}</notes>\n", self.notes).as_bytes())?;

        self.geo_info.write(&mut writer)?;
        self.colors.write(&mut writer)?;
        self.barrier.write(&mut writer)?;
        self.symbols.write(&mut writer)?;
        self.parts.write(&mut writer, &transform)?;

        writer.write_all(self.templates_and_view.as_bytes())?;
        writer.write_all("</barrier>\n</map>".as_bytes())?;

        Ok(())
    }
}
