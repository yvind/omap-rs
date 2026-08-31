use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{
    Cmyk, Color, ColorComponent, ColorId, ColorKey, MixedColor, MixedColorId, Rgb, SpotColor,
    SpotColorId, color::ColorParseReturn,
};
use crate::arena::Arena;
use crate::utils::{UnitF64, try_get_attr_raw};
use crate::{Error, OmapSection, Result};

/// An ordered set of map colors.
///
/// Position in the set is the color's priority, and the priority is what the
/// `.omap` format stores. A [`ColorId`] is independent of that: it keeps naming
/// the same color across [`ColorSet::swap`], [`ColorSet::insert`] and
/// [`ColorSet::remove_at`], and stops resolving once that color is removed.
///
/// A [`ColorId`] left over from a removed color contributes no color when the
/// map is written, exactly as a dangling weak reference did.
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorSet(Arena<Color, ColorKey>);

impl ColorSet {
    /// Create a new [`ColorSet`]
    pub fn new() -> Self {
        Self(Arena::new())
    }

    /// Get the number of colors in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the color set contains no colors.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Swap the priority of two colors. Both handles stay valid.
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

    /// Append a new color with the lowest priority.
    pub fn push(&mut self, color: impl Into<Color>) -> ColorId {
        ColorId(self.0.push(color.into()))
    }

    /// Insert a new color into the `ColorSet` with priority `index`, fails if `index > self.len()`
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if `index` is greater than the number of colors.
    pub fn insert(&mut self, index: usize, color: impl Into<Color>) -> Result<ColorId> {
        if index > self.len() {
            return Err(Error::ColorError);
        }
        Ok(ColorId(self.0.insert(index, color.into())))
    }

    /// Remove a color by its priority index.
    pub fn remove_at(&mut self, priority: usize) -> Option<Color> {
        self.0.remove_at(priority)
    }

    /// Remove a color by its handle.
    pub fn remove(&mut self, id: ColorId) -> Option<Color> {
        self.0.remove(id.0)
    }

    /// Returns `true` if `id` names a color still in this set.
    pub fn contains(&self, id: ColorId) -> bool {
        self.0.contains(id.0)
    }

    /// Get a color by its handle.
    pub fn get(&self, id: ColorId) -> Option<&Color> {
        self.0.get(id.0)
    }

    /// Mutably get a color by its handle.
    pub fn get_mut(&mut self, id: ColorId) -> Option<&mut Color> {
        self.0.get_mut(id.0)
    }

    /// Get a spot color by its handle. Cannot return a mixed color.
    pub fn spot_color(&self, id: SpotColorId) -> Option<&SpotColor> {
        match self.0.get(id.0) {
            Some(Color::SpotColor(color)) => Some(color),
            _ => None,
        }
    }

    /// Mutably get a spot color by its handle.
    pub fn spot_color_mut(&mut self, id: SpotColorId) -> Option<&mut SpotColor> {
        match self.0.get_mut(id.0) {
            Some(Color::SpotColor(color)) => Some(color),
            _ => None,
        }
    }

    /// Get a mixed color by its handle. Cannot return a spot color.
    pub fn mixed_color(&self, id: MixedColorId) -> Option<&MixedColor> {
        match self.0.get(id.0) {
            Some(Color::MixedColor(color)) => Some(color),
            _ => None,
        }
    }

    /// Mutably get a mixed color by its handle.
    pub fn mixed_color_mut(&mut self, id: MixedColorId) -> Option<&mut MixedColor> {
        match self.0.get_mut(id.0) {
            Some(Color::MixedColor(color)) => Some(color),
            _ => None,
        }
    }

    /// Narrow a handle to a spot color, or `None` if it names a mixed color.
    ///
    /// Widening a handle is a [`From`]; narrowing one needs the set, because
    /// the kind lives on the color rather than on the handle.
    pub fn spot_id(&self, id: ColorId) -> Option<SpotColorId> {
        matches!(self.get(id)?, Color::SpotColor(_)).then_some(SpotColorId(id.0))
    }

    /// Narrow a handle to a mixed color, or `None` if it names a spot color.
    pub fn mixed_id(&self, id: ColorId) -> Option<MixedColorId> {
        matches!(self.get(id)?, Color::MixedColor(_)).then_some(MixedColorId(id.0))
    }

    /// Get a color by its priority index.
    pub fn color_by_priority(&self, priority: usize) -> Option<&Color> {
        self.0.get_at(priority)
    }

    /// Mutably get a color by its priority index.
    pub fn color_by_priority_mut(&mut self, priority: usize) -> Option<&mut Color> {
        self.0.get_at_mut(priority)
    }

    /// Get a handle to the color with the given priority index.
    pub fn id_by_priority(&self, priority: usize) -> Option<ColorId> {
        Some(ColorId(self.0.id_at(priority)?))
    }

    /// Get the priority index of a color, or `None` if it is not in this set.
    pub fn priority_of(&self, id: ColorId) -> Option<usize> {
        self.0.position(id.0)
    }

    /// Get the first color with an exact name match.
    pub fn color_by_name(&self, name: &str) -> Option<&Color> {
        self.0.values().find(|color| color.name() == name)
    }

    /// Get a handle to the first color with an exact name match.
    pub fn id_by_name(&self, name: &str) -> Option<ColorId> {
        self.iter()
            .find(|(_, color)| color.name() == name)
            .map(|(id, _)| id)
    }

    /// Get the effective CMYK value of a color in this set.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if `id` is not in this set or the color
    /// definition cannot produce a CMYK value.
    pub fn cmyk(&self, id: ColorId) -> Result<Cmyk> {
        self.get(id).ok_or(Error::ColorError)?.cmyk(self)
    }

    /// Get the effective RGB value of a color in this set.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if `id` is not in this set or the color
    /// definition cannot produce an RGB value.
    pub fn rgb(&self, id: ColorId) -> Result<Rgb> {
        self.get(id).ok_or(Error::ColorError)?.rgb(self)
    }

    /// Iterate over the colors and their handles, in priority order.
    pub fn iter(&self) -> impl Iterator<Item = (ColorId, &Color)> {
        self.0.iter().map(|(raw, color)| (ColorId(raw), color))
    }

    /// Iterate over the colors in priority order.
    pub fn values(&self) -> impl Iterator<Item = &Color> {
        self.0.values()
    }

    /// Mutably iterate over the colors, in no particular order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Color> {
        self.0.values_mut()
    }

    /// Iterate over handles to every color, in priority order.
    pub fn ids(&self) -> impl Iterator<Item = ColorId> {
        self.iter().map(|(id, _)| id)
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

        // Colors are pushed in priority order first so that a mixed color's
        // components can be resolved by priority afterwards.
        let mut pending = Vec::with_capacity(colors_and_components.len());
        for color_parse_return in colors_and_components {
            match color_parse_return {
                ColorParseReturn::Spot { color, priority } => {
                    pending.push((priority, Color::SpotColor(color), Vec::new()));
                }
                ColorParseReturn::Mix {
                    color,
                    priority,
                    components,
                } => pending.push((priority, Color::MixedColor(color), components)),
            }
        }
        pending.sort_by_key(|(priority, _, _)| *priority);

        let mut color_set = Self(Arena::with_capacity(pending.len()));
        let mut all_components = Vec::with_capacity(pending.len());
        for (_, color, components) in pending {
            let _id = color_set.push(color);
            all_components.push(components);
        }

        for (position, components) in all_components.into_iter().enumerate() {
            if components.is_empty() {
                continue;
            }
            let resolved: Vec<ColorComponent> = components
                .into_iter()
                .filter_map(|(priority, factor)| {
                    let priority = usize::try_from(priority).ok()?;
                    let color = color_set.spot_id(color_set.id_by_priority(priority)?)?;
                    Some(ColorComponent {
                        factor: UnitF64::clamped_from(factor),
                        color,
                    })
                })
                .collect();
            if let Some(Color::MixedColor(color)) = color_set.color_by_priority_mut(position) {
                color.components = resolved;
            }
        }

        Ok(color_set)
    }

    pub(crate) fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Start(
            BytesStart::new("colors").with_attributes([("count", self.len().to_string().as_str())]),
        ))?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        for (priority, color) in self.values().enumerate() {
            match color {
                Color::SpotColor(color) => color.write(writer, priority)?,
                Color::MixedColor(color) => color.write(writer, priority, self)?,
            }
            writer.get_mut().write_all(b"\n".as_slice())?;
        }
        writer.write_event(Event::End(BytesEnd::new("colors")))?;
        Ok(())
    }
}
