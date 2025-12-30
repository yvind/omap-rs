use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
    str::FromStr,
};

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

use super::{Cmyk, Color};
use crate::editor::{Error, Result};

#[derive(Debug, Clone)]
pub struct ColorSet(Vec<Rc<RefCell<Color>>>);

impl ColorSet {
    /// Add a simple color to the end of the color list
    pub fn push_color(&mut self, name: String, cmyk: Cmyk, opacity: f64) {
        let [c, m, y, k] = cmyk.as_rounded_fractions(2);

        let def = format!(
            "<color priority=\"{}\" name=\"{name}\" c=\"{c}\" m=\"{m}\" y=\"{y}\" k=\"{k}\" opacity=\"{opacity}\"><cmyk method=\"custom\"/></color>\n",
            self.num_colors()
        );

        self.0.push(Rc::new(RefCell::new(Color::new(
            name,
            cmyk,
            def,
            opacity,
            self.num_colors(),
        ))));
    }

    pub fn num_colors(&self) -> usize {
        self.0.len()
    }

    pub fn get_color_by_id(&self, id: usize) -> Option<Ref<'_, Color>> {
        self.0
            .iter()
            .filter_map(|c| c.try_borrow().ok())
            .find(|c| c.get_id() == id)
    }

    pub(crate) fn get_weak_color_by_id(&self, id: usize) -> Option<Weak<RefCell<Color>>> {
        self.0
            .iter()
            .find(|&c| match c.try_borrow().ok() {
                Some(c) => c.get_id() == id,
                None => false,
            })
            .map(Rc::downgrade)
    }

    /// Get the first color with an exact name match
    pub fn get_color_by_name(&self, name: &str) -> Option<Ref<'_, Color>> {
        self.0
            .iter()
            .filter_map(|c| c.try_borrow().ok())
            .find(|c| c.get_name() == name)
    }

    /// Access the colors through an iterator
    pub fn iter(&self) -> impl Iterator<Item = Result<Ref<'_, Color>>> {
        self.0.iter().map(|s| {
            let s = s.try_borrow()?;
            Ok(s)
        })
    }

    /// Access the mutable colors through an iterator
    pub fn iter_mut(&mut self) -> impl Iterator<Item = Result<RefMut<'_, Color>>> {
        self.0.iter().map(|s| {
            let s = s.try_borrow_mut()?;
            Ok(s)
        })
    }
}

impl ColorSet {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart,
    ) -> Result<ColorSet> {
        let mut num_colors = 0;
        for attr in element.attributes().filter_map(std::result::Result::ok) {
            if attr.key.local_name().as_ref() == b"count" {
                num_colors = usize::from_str(&attr.unescape_value()?)?
            }
        }

        let mut colors = Vec::with_capacity(num_colors);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"color") {
                        colors.push(Rc::new(RefCell::new(Color::parse(reader, &bytes_start)?)))
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
            Rc::into_inner(color)
                .ok_or(Error::ParseOmapFileError(
                    "Stray strong references to the colors somewhere".to_string(),
                ))?
                .into_inner()
                .write(writer)?;
        }

        writer.write_all("</colors>\n".as_bytes())?;
        Ok(())
    }
}
