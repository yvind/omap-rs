use quick_xml::events::BytesStart;

use super::Cmyk;
use crate::editor::Result;

#[derive(Debug, Clone)]
pub struct Color {
    name: String,
    priority: usize,
    cmyk: Cmyk,
    xml_def: String,
}

impl Color {
    pub(super) fn new(name: String, cmyk: Cmyk, xml_def: String, priority: usize) -> Color {
        Color {
            name,
            priority,
            cmyk,
            xml_def,
        }
    }

    /// Get the name of the color
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the cmyk values of the color
    pub fn get_cmyk(&self) -> &Cmyk {
        &self.cmyk
    }

    /// Get the xml definition of the color
    pub fn get_xml_definition(&self) -> &str {
        &self.xml_def
    }
}

impl Color {
    pub(super) fn parse_color(element: &BytesStart) -> Result<Color> {
        todo!()
    }

    pub(super) fn write<W: std::io::Write>(
        self,
        write: &mut W,
    ) -> std::result::Result<(), std::io::Error> {
        write.write_all(self.xml_def.as_bytes())
    }
}
