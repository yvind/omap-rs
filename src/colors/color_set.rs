use std::{cell::RefCell, rc::Rc};

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{Color, ColorComponent, WeakColor, color::ColorParseReturn};
use crate::utils::{UnitF64, try_get_attr_raw};
use crate::{Error, OmapSection, Result};

/// An ordered set of map colors.
///
/// The order of the [`Color`] values in the [`Vec`] determines their priority.
/// Move colors to change priority, for example with `color_set.swap(2, 5)`.
///
/// Deleting a color drops its allocation if there are no outstanding [`Rc`]
/// references. Symbols and [`ColorComponent`] values only hold
/// [`std::rc::Weak`] references. A dangling weak reference contributes no color
/// when the map is written.
#[derive(Debug, Default)]
pub struct ColorSet(Vec<Color>);

impl ColorSet {
    /// Create a new [`ColorSet`]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Get the number of colors in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Swap the priority of two colors
    ///
    /// # Errors
    ///
    /// Returns an error if either of the arguments are out of bounds
    pub fn swap(&mut self, first: usize, second: usize) -> Result<()> {
        if first < self.len() && second < self.len() {
            self.0.swap(first, second);
            Ok(())
        } else {
            Err(Error::MissingColorId)
        }
    }

    /// Returns `true` if the color set contains no colors.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Append a new color with the lowest priority. Returns a weak pointer to the color.
    pub fn push(&mut self, color: impl Into<Color>) -> WeakColor {
        let color = color.into();
        let weak = color.downgrade();
        self.0.push(color);
        weak
    }

    /// Remove a color by its priority index.
    pub fn remove(&mut self, index: usize) -> Option<Color> {
        if index < self.len() {
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    /// Insert a new color into the `ColorSet` with priority `index`, fails if `index > self.len()`
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if `index` is greater than the number of colors.
    pub fn insert(&mut self, index: usize, color: impl Into<Color>) -> Result<WeakColor> {
        if index > self.len() {
            return Err(Error::ColorError);
        }
        let color = color.into();
        let weak = color.downgrade();
        self.0.insert(index, color);
        Ok(weak)
    }

    /// Get a color by its priority index.
    pub fn color_by_priority(&self, priority: usize) -> Option<&Color> {
        self.0.get(priority)
    }

    /// Get a weak reference to a color by its priority index.
    pub fn weak_color_by_priority(&self, priority: usize) -> Option<WeakColor> {
        self.color_by_priority(priority).map(|c| c.downgrade())
    }

    /// Get the first color with an exact name match
    ///
    /// # Errors
    ///
    /// Returns an error if a color cannot be borrowed because it is mutably borrowed somewhere else
    pub fn color_by_name(&self, name: &str) -> Result<Option<&Color>> {
        for color in &self.0 {
            match color {
                Color::SpotColor(ref_cell) => {
                    if ref_cell.try_borrow()?.name() == name {
                        return Ok(Some(color));
                    }
                }
                Color::MixedColor(ref_cell) => {
                    if ref_cell.try_borrow()?.name() == name {
                        return Ok(Some(color));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Get the priority index of a specific color in the set (by pointer identity).
    pub fn priority_of_color(&self, color: &Color) -> Option<usize> {
        self.iter().position(|c| c == color)
    }

    /// Get the priority index of a specific color in the set (by pointer identity).
    pub fn priority_of_weak_color(&self, color: &WeakColor) -> Option<usize> {
        self.iter_weak().position(|c| &c == color)
    }

    /// Access the colors through an iterator
    pub fn iter(&self) -> impl Iterator<Item = &Color> {
        self.0.iter()
    }

    /// Iterate over weak references to the colors.
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
    ) -> Result<Self> {
        let num_colors = try_get_attr_raw(element, "count")?.ok_or(Error::ColorError)?;
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
                    return Err(Error::UnexpectedEof(OmapSection::ColorSet));
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
                    spot_colors.push((Rc::new(RefCell::new(color)), priority));
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
        parsed_colors.sort_by_key(|a| a.1);

        Ok(Self(parsed_colors.into_iter().map(|(c, _)| c).collect()))
    }

    pub(crate) fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Start(
            BytesStart::new("colors").with_attributes([("count", self.len().to_string().as_str())]),
        ))?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        for (priority, color) in self.0.iter().enumerate() {
            match color {
                Color::SpotColor(ref_cell) => ref_cell.try_borrow()?.write(writer, priority)?,
                Color::MixedColor(ref_cell) => {
                    ref_cell.try_borrow()?.write(writer, priority, self)?;
                }
            }
            writer.get_mut().write_all(b"\n".as_slice())?;
        }
        writer.write_event(Event::End(BytesEnd::new("colors")))?;
        Ok(())
    }
}
