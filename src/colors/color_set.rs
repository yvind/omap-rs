use std::{cell::RefCell, rc::Rc};

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{Color, ColorComponent, WeakColor, color::ColorParseReturn};
use crate::utils::{UnitF64, try_get_attr};
use crate::{Error, Result};

/// The order of the [Color]s in the [Vec] is the order of priority
/// Move the colors around to change priority, f.ex. using `color_set.0.swap(2, 5)`
/// Deleting a [Color] from the [ColorSet] will drop that colors allocation (if no outstanding [Rc]s have been made)
/// as [crate::editor::symbols::Symbol]s and [ColorComponent]s in [super::MixedColor] only have [std::rc::Weak] references
/// If a weak reference is referencing a deleted color at the time of writing to file, no color will be used
#[derive(Debug, Clone)]
pub struct ColorSet(pub Vec<Color>);

impl ColorSet {
    pub fn num_colors(&self) -> usize {
        self.0.len()
    }

    pub fn get_color_by_priority(&self, priority: usize) -> Option<&Color> {
        if self.num_colors() >= priority {
            None
        } else {
            Some(&self.0[priority])
        }
    }

    pub fn get_weak_color_by_priority(&self, priority: usize) -> Option<WeakColor> {
        self.get_color_by_priority(priority).map(|c| c.downgrade())
    }

    /// Get the first color with an exact name match
    pub fn get_color_by_name(&self, name: &str) -> Option<&Color> {
        self.0.iter().find(|c| match c {
            Color::SpotColor(ref_cell) => match ref_cell.try_borrow() {
                Ok(c) => c.get_name() == name,
                Err(_) => false,
            },
            Color::MixedColor(ref_cell) => match ref_cell.try_borrow() {
                Ok(c) => c.get_name() == name,
                Err(_) => false,
            },
        })
    }

    pub fn get_id_of_color(&self, color: &Color) -> Option<usize> {
        self.iter()
            .enumerate()
            .find(|(_, c)| match (color, c) {
                (Color::SpotColor(c1), Color::SpotColor(c2)) => c1.as_ptr() == c2.as_ptr(),
                (Color::MixedColor(c1), Color::MixedColor(c2)) => c1.as_ptr() == c2.as_ptr(),
                _ => false,
            })
            .map(|(i, _)| i)
    }

    /// Access the colors through an iterator
    pub fn iter(&self) -> impl Iterator<Item = &Color> {
        self.0.iter()
    }

    pub fn iter_weak(&self) -> impl Iterator<Item = WeakColor> {
        self.0.iter().map(|c| c.downgrade())
    }

    /// Access the mutable colors through an iterator
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Color> {
        self.0.iter_mut()
    }
}

impl ColorSet {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
    ) -> Result<ColorSet> {
        let num_colors = try_get_attr(element, "count").ok_or(Error::ColorError)?;
        let mut colors_and_components = Vec::with_capacity(num_colors);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"color") {
                        colors_and_components.push(Color::parse(reader, &bytes_start)?);
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

        // Now that all colors have been identified we can finish parsing the references and completing all colors
        let mut spot_colors = Vec::with_capacity(num_colors);
        let mut mixed_colors = Vec::with_capacity(num_colors);
        let mut parsed_colors = Vec::with_capacity(num_colors);

        for color_parse_return in colors_and_components {
            match color_parse_return {
                ColorParseReturn::Spot { color, priority } => {
                    spot_colors.push((Rc::new(RefCell::new(color)), priority))
                }
                ColorParseReturn::Mix {
                    color,
                    priority,
                    components,
                } => mixed_colors.push((color, priority, components)),
            }
        }

        for (mut color, priority, components) in mixed_colors {
            for (id, factor) in components {
                if id < 0 || id >= num_colors as i32 {
                    continue;
                }
                let id = id as usize;

                if let Some((c, _)) = spot_colors.iter().find(|(_, prio)| *prio == id) {
                    color.components.push(ColorComponent {
                        factor: UnitF64::clamped_from(factor),
                        color: Rc::downgrade(c),
                    });
                }
            }
            parsed_colors.push((Color::MixedColor(Rc::new(RefCell::new(color))), priority));
        }
        parsed_colors.extend(
            spot_colors
                .into_iter()
                .map(|(s, p)| (Color::SpotColor(s), p)),
        );
        parsed_colors.sort_by(|a, b| a.1.cmp(&b.1));

        Ok(ColorSet(
            parsed_colors.into_iter().map(|(c, _)| c).collect(),
        ))
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Start(
            BytesStart::new("colors")
                .with_attributes([("count", self.num_colors().to_string().as_str())]),
        ))?;

        for (priority, color) in self.0.iter().enumerate() {
            match color {
                Color::SpotColor(ref_cell) => ref_cell.try_borrow()?.write(writer, priority)?,
                Color::MixedColor(ref_cell) => {
                    ref_cell.try_borrow()?.write(writer, priority, &self)?
                }
            };
        }
        writer.write_event(Event::End(BytesEnd::new("colors")))?;
        Ok(())
    }
}
