use quick_xml::{Reader, events::Event};

use super::{Cmyk, Color};
use crate::editor::{Error, Result};

#[derive(Debug, Clone)]
pub struct ColorSet(Vec<Color>);

impl ColorSet {
    /// Add a simple color to the end of the color list
    pub fn push_color(&mut self, name: String, cmyk: Cmyk, opacity: f64) {
        let [c, m, y, k] = cmyk.as_rounded_fractions(2);

        let def = format!(
            "<color priority=\"{}\" name=\"{name}\" c=\"{c}\" m=\"{m}\" y=\"{y}\" k=\"{k}\" opacity=\"{opacity}\"><cmyk method=\"custom\"/></color>\n",
            self.num_colors()
        );

        self.0
            .push(Color::new(name, cmyk, def, self.num_colors(), opacity));
    }

    pub fn num_colors(&self) -> usize {
        self.0.len()
    }

    pub fn get_color_by_priority(&self, index: usize) -> Option<&Color> {
        if index >= self.num_colors() {
            None
        } else {
            Some(&self.0[index])
        }
    }

    /// Get the first color with an exact name match
    pub fn get_color_by_name(&self, name: &str) -> Option<&Color> {
        self.0.iter().find(|&c| c.get_name() == name)
    }

    /// Access the colors through an iterator
    pub fn iter(&self) -> std::slice::Iter<'_, Color> {
        self.0.iter()
    }
}

impl ColorSet {
    pub(crate) fn parse<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<ColorSet> {
        let mut buf = Vec::new();

        let mut colors = Vec::new();

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"color") {
                        colors.push(Color::parse_color(reader, &bytes_start)?)
                    }
                }
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"colors") {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in color set parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }

        Ok(ColorSet(colors))
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(format!("<colors count=\"{}\">\n", self.num_colors()).as_bytes())?;

        for color in self.0 {
            color.write(writer)?;
        }

        writer.write_all("</colors>\n".as_bytes())?;
        Ok(())
    }
}
