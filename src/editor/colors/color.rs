use quick_xml::{Reader, events::BytesStart};
use std::{io::BufRead, str::FromStr};

use super::Cmyk;
use crate::editor::Result;

#[derive(Debug, Clone)]
pub struct Color {
    name: String,
    priority: usize,
    opacity: f64,
    cmyk: Cmyk,
    xml_def: String,
}

impl Color {
    pub(super) fn new(
        name: String,
        cmyk: Cmyk,
        xml_def: String,
        priority: usize,
        opacity: f64,
    ) -> Color {
        Color {
            name,
            priority,
            cmyk,
            opacity,
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
    pub(super) fn parse_color<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart,
    ) -> Result<Color> {
        let mut xml_def = format!("<color{}>", std::str::from_utf8(element.attributes_raw())?);
        let mut priority = usize::MAX;
        let mut name = String::new();
        let mut cmyk = Cmyk::new(0., 0., 0., 0.);
        let mut opacity = 1.;

        for attr in element.attributes() {
            let attr = attr?;

            match attr.key.local_name().as_ref() {
                b"priority" => {
                    priority = usize::from_str(std::str::from_utf8(attr.value.as_ref())?)?
                }
                b"name" => name.push_str(std::str::from_utf8(&attr.value)?),
                b"c" => cmyk.c = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"m" => cmyk.m = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"y" => cmyk.y = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"k" => cmyk.k = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                b"opacity" => opacity = f64::from_str(std::str::from_utf8(attr.value.as_ref())?)?,
                _ => (),
            }
        }

        let _ = reader.stream().read_line(&mut xml_def);

        Ok(Color {
            name,
            priority,
            cmyk,
            xml_def,
            opacity,
        })
    }

    pub(super) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(self.xml_def.as_bytes())?;
        Ok(())
    }
}
