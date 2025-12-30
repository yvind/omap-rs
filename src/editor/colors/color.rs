use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use std::str::FromStr;

use super::Cmyk;
use crate::editor::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    name: String,
    opacity: f64,
    cmyk: Cmyk,
    xml_def: String,
    id: usize,
}

impl Color {
    pub(super) fn new(name: String, cmyk: Cmyk, xml_def: String, opacity: f64, id: usize) -> Color {
        Color {
            name,
            cmyk,
            opacity,
            xml_def,
            id,
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

    pub fn get_id(&self) -> usize {
        self.id
    }
}

impl Color {
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart,
    ) -> Result<Color> {
        let mut xml_def = format!("<color{}>", std::str::from_utf8(element.attributes_raw())?);
        let mut name = String::new();
        let mut cmyk = Cmyk::new(0., 0., 0., 0.);
        let mut opacity = 1.;
        let mut id = usize::MAX;

        for attr in element.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"name" => name.push_str(std::str::from_utf8(&attr.value)?),
                b"c" => cmyk.c = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"m" => cmyk.m = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"y" => cmyk.y = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"k" => cmyk.k = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"opacity" => opacity = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"priority" => id = usize::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                _ => (),
            }
        }

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    xml_def.push_str(
                        format!(
                            "<{}{}>",
                            std::str::from_utf8(bytes_start.local_name().as_ref())?,
                            std::str::from_utf8(bytes_start.attributes_raw())?
                        )
                        .as_str(),
                    );
                }
                Event::End(bytes_end) => match bytes_end.local_name().as_ref() {
                    b"color" => {
                        xml_def.push_str("</color>");
                        break;
                    }
                    name => xml_def.push_str(format!("</{}>", std::str::from_utf8(name)?).as_str()),
                },
                Event::Empty(bytes_start) => {
                    xml_def.push_str(
                        format!(
                            "<{}{}/>",
                            std::str::from_utf8(bytes_start.local_name().as_ref())?,
                            std::str::from_utf8(bytes_start.attributes_raw())?
                        )
                        .as_str(),
                    );
                }
                Event::Text(bytes_text) => {
                    xml_def.push_str(&bytes_text.xml_content()?);
                }
                Event::Eof => return Err(Error::ParseOmapFileError("Early EOF".to_string())),
                _ => (),
            }
        }

        if id == usize::MAX {
            return Err(Error::ParseOmapFileError(
                "Could not parse color".to_string(),
            ));
        }

        Ok(Color {
            name,
            cmyk,
            xml_def,
            opacity,
            id,
        })
    }

    pub(super) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(self.xml_def.as_bytes())?;
        Ok(())
    }
}
