use quick_xml::{Reader, events::BytesStart};

use super::{Cmyk, Color};
use crate::editor::Result;

#[derive(Debug, Clone)]
pub struct ColorSet(Vec<Color>);

impl ColorSet {
    /// Add a simple color to the end of the color list
    pub fn push_color(&mut self, name: String, cmyk: Cmyk) {
        let [c, m, y, k] = cmyk.as_rounded_fractions(2);

        let def = format!(
            "<color priority=\"{}\" name=\"{name}\" c=\"{c}\" m=\"{m}\" y=\"{y}\" k=\"{k}\" opacity=\"1\"><cmyk method=\"custom\"/></color>\n",
            self.num_colors()
        );

        self.0.push(Color::new(name, cmyk, def, self.num_colors()));
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
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart,
    ) -> Result<ColorSet> {
        todo!()
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
