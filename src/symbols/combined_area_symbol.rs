use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{AreaSymbol, LineSymbol, PublicOrPrivateSymbol, SymbolCommon, SymbolSet};
use crate::{
    Code, Error, OmapSection, Result,
    colors::{ColorId, ColorSet},
    notes,
    symbols::{AreaOrLineSymbol, PathSymbolId, Symbol, SymbolId},
    utils::{parse_attr, try_get_attr_raw},
};

/// A combined area symbol composed of multiple sub-symbols.
#[derive(Debug, Clone)]
pub struct CombinedAreaSymbol {
    /// Common symbol properties.
    pub common: SymbolCommon,
    /// The component parts of this combined symbol.
    /// Public components are added through [`SymbolSet::add_area_component`],
    /// which rejects any component that would make the definition cyclic.
    parts: Vec<PublicOrPrivateSymbol<PathSymbolId, AreaOrLineSymbol>>,
}

impl CombinedAreaSymbol {
    /// Iterate through the symbol component of the symbol
    pub fn components(
        &self,
    ) -> impl Iterator<Item = &PublicOrPrivateSymbol<PathSymbolId, AreaOrLineSymbol>> {
        self.parts.iter()
    }

    /// Iterate over only the public components.
    pub fn public_components(&self) -> impl Iterator<Item = PathSymbolId> {
        self.parts.iter().filter_map(|part| match part {
            PublicOrPrivateSymbol::Public(id) => Some(*id),
            PublicOrPrivateSymbol::Private(_) => None,
        })
    }

    /// Remove and return the symbol component at position `index` in the component vec.
    ///
    /// This preserves the order of the components.
    ///
    /// Note: Because this shifts over the remaining elements, it has a worst-case performance of O(n). If you don't need the order of elements to be preserved, use [`Self::swap_remove_component`] instead.
    pub fn remove_component(
        &mut self,
        index: usize,
    ) -> Option<PublicOrPrivateSymbol<PathSymbolId, AreaOrLineSymbol>> {
        if self.parts.len() > index {
            Some(self.parts.remove(index))
        } else {
            None
        }
    }

    /// Removes a component from the symbol and returns it.
    ///
    /// The last component is moved to the removed components index.
    ///
    /// This does not preserve ordering of the remaining components, but is O(1). If you need to preserve the component order, use [`Self::remove_component()`] instead.
    pub fn swap_remove_component(
        &mut self,
        index: usize,
    ) -> Option<PublicOrPrivateSymbol<PathSymbolId, AreaOrLineSymbol>> {
        if self.parts.len() > index {
            Some(self.parts.swap_remove(index))
        } else {
            None
        }
    }

    pub(crate) fn push_component(
        &mut self,
        component: PublicOrPrivateSymbol<PathSymbolId, AreaOrLineSymbol>,
    ) {
        self.parts.push(component);
    }

    /// Create a new empty combined area symbol with the given code and name.
    pub fn new(code: Code, name: impl Into<String>) -> Self {
        let common = SymbolCommon {
            code,
            name: name.into(),
            ..Default::default()
        };
        Self {
            common,
            parts: Vec::new(),
        }
    }

    /// Get the display name of this combined area symbol.
    pub fn name(&self) -> &str {
        &self.common.name
    }

    /// Get the number of components in this combined symbol.
    pub fn num_components(&self) -> usize {
        self.parts.len()
    }

    /// Mark as a helper symbol (builder-style).
    pub fn as_helper_symbol(mut self) -> Self {
        self.common.is_helper_symbol = true;
        self
    }

    /// Get the minimum area (in paper dimensions mm²) among all area sub-symbols.
    ///
    /// Takes the [`SymbolSet`] that owns the public components. A component no
    /// longer in the set contributes nothing.
    pub fn minimum_area(&self, symbol_set: &SymbolSet) -> f64 {
        let mut min = f64::MAX;
        for part in &self.parts {
            let area = match part {
                PublicOrPrivateSymbol::Public(PathSymbolId::Area(id)) => symbol_set
                    .area_symbol(*id)
                    .map_or(0., |symbol| symbol.minimum_area.get()),
                PublicOrPrivateSymbol::Public(PathSymbolId::CombinedArea(id)) => symbol_set
                    .combined_area_symbol(*id)
                    .map_or(0., |symbol| symbol.minimum_area(symbol_set)),
                PublicOrPrivateSymbol::Private(AreaOrLineSymbol::Area(symbol)) => {
                    symbol.minimum_area.get()
                }
                _ => 0.,
            };
            if area > 0. {
                min = min.min(area);
            }
        }
        if min == f64::MAX { 0. } else { min }
    }

    /// Every color used in this symbol definition.
    ///
    /// Takes the [`SymbolSet`] that owns the public components. A component no
    /// longer in the set contributes no colors.
    pub fn colors(&self, symbol_set: &SymbolSet) -> Vec<ColorId> {
        let mut colors = Vec::new();

        for component in self.components() {
            match component {
                PublicOrPrivateSymbol::Public(id) => {
                    if let Some(symbol) = symbol_set.get(SymbolId::from(*id)) {
                        colors.extend(symbol.colors(symbol_set));
                    }
                }
                PublicOrPrivateSymbol::Private(symbol) => match symbol {
                    AreaOrLineSymbol::Area(symbol) => colors.extend(symbol.colors()),
                    AreaOrLineSymbol::Line(symbol) => colors.extend(symbol.colors()),
                },
            }
        }

        colors
    }

    /// Does this symbol reference `other`, directly or through a component?
    ///
    /// Takes the [`SymbolSet`] that owns the public components.
    pub fn contains_symbol(&self, symbol_set: &SymbolSet, other: SymbolId) -> bool {
        self.public_components().any(|component| {
            if SymbolId::from(component) == other {
                return true;
            }
            match symbol_set.get(SymbolId::from(component)) {
                Some(Symbol::CombinedArea(symbol)) => symbol.contains_symbol(symbol_set, other),
                Some(Symbol::CombinedLine(symbol)) => symbol.contains_symbol(symbol_set, other),
                _ => false,
            }
        })
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<(Self, Vec<usize>)> {
        let mut common = attributes;
        let mut parts: Vec<PublicOrPrivateSymbol<PathSymbolId, AreaOrLineSymbol>> = Vec::new();
        let mut public_component_ids: Vec<usize> = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"description" => common.description = notes::parse(reader)?,
                    b"combined_symbol" => {
                        // num_parts attribute is informational, we parse dynamically
                    }
                    b"part" => {
                        let is_private = try_get_attr_raw(&e, "private")?.unwrap_or(false);
                        if is_private {
                            // Parse the private sub-symbol
                            let sym = Self::parse_private_part(reader, color_set)?;
                            parts.push(PublicOrPrivateSymbol::Private(sym));
                        } else {
                            let symbol_index = try_get_attr_raw::<i32>(&e, "symbol")?;
                            // Record the public component ID for later resolution.
                            // Mapper uses -1 for unknown / empty public parts; skip those.
                            if let Some(symbol_index) = symbol_index.filter(|&id| id >= 0) {
                                public_component_ids.push(symbol_index as usize);
                            }
                            // Don't push to parts here - will be resolved by symbol_set after all symbols are loaded
                        }
                    }
                    b"icon" => common.custom_icon = try_get_attr_raw(&e, "src")?,
                    _ => {}
                },
                Event::End(e) if e.local_name().as_ref() == b"symbol" => {
                    break;
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::CombinedAreaSymbol));
                }
                _ => {}
            }
        }

        Ok((Self { common, parts }, public_component_ids))
    }

    fn parse_private_part<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
    ) -> Result<AreaOrLineSymbol> {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) if e.local_name().as_ref() == b"symbol" => {
                    let sym_type: u8 = try_get_attr_raw(&e, "type")?.unwrap_or(0);
                    let mut sub_common = SymbolCommon::default();
                    for attr in e.attributes().filter_map(std::result::Result::ok) {
                        match attr.key.local_name().as_ref() {
                            b"name" => {
                                sub_common.name =
                                    parse_attr(attr, e.decoder()).unwrap_or(sub_common.name);
                            }
                            b"code" => {
                                sub_common.code =
                                    crate::utils::parse_attr_raw(attr.value).unwrap_or_default();
                            }
                            _ => {}
                        }
                    }
                    match sym_type {
                        2 => {
                            let line = LineSymbol::parse(reader, color_set, sub_common)?;
                            // Skip to end of part
                            Self::skip_to_end_of_part(reader)?;
                            return Ok(AreaOrLineSymbol::Line(Box::new(line)));
                        }
                        4 => {
                            let area = AreaSymbol::parse(reader, color_set, sub_common)?;
                            Self::skip_to_end_of_part(reader)?;
                            return Ok(AreaOrLineSymbol::Area(Box::new(area)));
                        }
                        _ => {
                            return Err(Error::UnknownPrivatePartSymbolType(sym_type));
                        }
                    }
                }
                Event::End(e) if e.local_name().as_ref() == b"part" => {
                    return Err(Error::EmptyPrivatePart);
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::PrivatePart));
                }
                _ => {}
            }
        }
    }

    fn skip_to_end_of_part<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<()> {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::End(e) if e.local_name().as_ref() == b"part" => {
                    return Ok(());
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::SkippedPart));
                }
                _ => {}
            }
        }
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
        color_set: &ColorSet,
        index: usize,
    ) -> Result<()> {
        let mut bs = BytesStart::new("symbol").with_attributes([
            ("type", "16"),
            ("code", self.common.code.to_string().as_str()),
            ("name", self.common.name.as_str()),
            ("id", index.to_string().as_str()),
        ]);
        if self.common.is_hidden {
            bs.push_attribute(("is_hidden", "true"));
        }
        if self.common.is_helper_symbol {
            bs.push_attribute(("is_helper_symbol", "true"));
        }
        if self.common.is_protected {
            bs.push_attribute(("is_protected", "true"));
        }
        writer.write_event(Event::Start(bs))?;

        if !self.common.description.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("description")))?;
            writer.write_event(Event::Text(BytesText::new(&self.common.description)))?;
            writer.write_event(Event::End(BytesEnd::new("description")))?;
        }

        let mut cs = BytesStart::new("combined_symbol");
        cs.push_attribute(("parts", self.parts.len().to_string().as_str()));
        writer.write_event(Event::Start(cs))?;

        for part in &self.parts {
            match part {
                PublicOrPrivateSymbol::Public(id) => {
                    let sym_index = symbol_set
                        .index_of(SymbolId::from(*id))
                        .map_or(-1, |index| index as i32);

                    writer.write_event(Event::Empty(
                        BytesStart::new("part")
                            .with_attributes([("symbol", sym_index.to_string().as_str())]),
                    ))?;
                }
                PublicOrPrivateSymbol::Private(path_sym) => {
                    writer.write_event(Event::Start(
                        BytesStart::new("part").with_attributes([("private", "true")]),
                    ))?;
                    match path_sym {
                        AreaOrLineSymbol::Line(line) => {
                            line.write(writer, color_set, None, false)?;
                        }
                        AreaOrLineSymbol::Area(area) => {
                            area.write(writer, color_set, None, false)?;
                        }
                    }
                    writer.write_event(Event::End(BytesEnd::new("part")))?;
                }
            }
        }

        writer.write_event(Event::End(BytesEnd::new("combined_symbol")))?;

        if let Some(icon) = &self.common.custom_icon {
            writer.write_event(Event::Empty(
                BytesStart::new("icon").with_attributes([("src", icon.as_str())]),
            ))?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbol")))?;
        Ok(())
    }
}
