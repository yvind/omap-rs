use std::cell::RefCell;
use std::collections::HashSet;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{AreaSymbol, LineSymbol, PublicOrPrivateSymbol, SymbolCommon, SymbolSet};
use crate::{
    Code, Error, OmapSection, Result,
    colors::ColorSet,
    symbols::{AreaOrLineSymbol, CombinedLineSymbol, WeakPathSymbol, WeakSymbol},
    utils::{parse_attr, try_get_attr_raw},
};

/// A combined area symbol composed of multiple sub-symbols.
#[derive(Debug, Clone)]
pub struct CombinedAreaSymbol {
    /// Common symbol properties.
    pub common: SymbolCommon,
    /// The component parts of this combined symbol.
    /// Be careful not to make circular symbol definitions (combined symbol A contains B which contains C which contains A)
    parts: Vec<PublicOrPrivateSymbol<WeakPathSymbol, AreaOrLineSymbol>>,
}

impl CombinedAreaSymbol {
    /// Iterate through the symbol component of the symbol
    pub fn components(
        &self,
    ) -> impl Iterator<Item = &PublicOrPrivateSymbol<WeakPathSymbol, AreaOrLineSymbol>> {
        self.parts.iter()
    }

    /// Iterate through the mutable symbol component of the symbol
    pub fn components_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut PublicOrPrivateSymbol<WeakPathSymbol, AreaOrLineSymbol>> {
        self.parts.iter_mut()
    }

    /// Remove and return the symbol component at position `index` in the component vec.
    /// This preserves the order of the components. O(N) run time
    pub fn remove_component(
        &mut self,
        index: usize,
    ) -> Option<PublicOrPrivateSymbol<WeakPathSymbol, AreaOrLineSymbol>> {
        if self.parts.len() > index {
            Some(self.parts.remove(index))
        } else {
            None
        }
    }

    /// Remove and return the symbol component at position `index` in the component vec.
    /// This does not preserve the order of the components. O(1) run time
    pub fn swap_remove_component(
        &mut self,
        index: usize,
    ) -> Option<PublicOrPrivateSymbol<WeakPathSymbol, AreaOrLineSymbol>> {
        if self.parts.len() > index {
            Some(self.parts.swap_remove(index))
        } else {
            None
        }
    }

    /// Adds a component to the symbol
    /// Fails if adding this component will create a cycle in the symbol component definitions
    ///
    /// The cycle detection relies on run time borrow checking, meaning that if any of the sub-symbols refcells
    /// are already being borrowed (through any of the .(try_)borrow(), .(try_)borrow_mut() functions) it fails and the component will not be added
    pub fn add_component(
        &mut self,
        new_component: PublicOrPrivateSymbol<WeakPathSymbol, AreaOrLineSymbol>,
    ) -> Result<()> {
        if matches!(
            new_component,
            PublicOrPrivateSymbol::Public(WeakPathSymbol::CombinedLine(_))
                | PublicOrPrivateSymbol::Public(WeakPathSymbol::CombinedArea(_))
        ) {
            self.parts.push(new_component);
            match self.contains_cycle() {
                Ok(true) => {
                    let _ = self.parts.pop();
                    Err(Error::CyclicSymbolDefinition)
                }
                Ok(false) => Ok(()),
                Err(e) => {
                    let _ = self.parts.pop();
                    Err(e)
                }
            }
        } else {
            self.parts.push(new_component);
            Ok(())
        }
    }

    /// Create a new empty combined area symbol with the given code and name.
    pub fn new(code: Code, name: impl Into<String>) -> CombinedAreaSymbol {
        let common = SymbolCommon {
            code,
            name: name.into(),
            ..Default::default()
        };
        CombinedAreaSymbol {
            common,
            parts: Vec::new(),
        }
    }

    /// Get the display name of this combined area symbol.
    pub fn get_name(&self) -> &str {
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
    /// The check fails if any child combined area symbols cannot be borrowed
    pub fn minimum_area(&self) -> Result<f64> {
        let mut min = f64::MAX;
        for s in self.parts.iter() {
            match s {
                PublicOrPrivateSymbol::Public(p) => {
                    if let WeakPathSymbol::Area(weak) = p
                        && let Some(area) = weak.upgrade()
                    {
                        let area_symbol = area.try_borrow()?;
                        if area_symbol.minimum_area.get() > 0. {
                            min = min.min(area_symbol.minimum_area.get());
                        }
                    }
                }
                PublicOrPrivateSymbol::Private(p) => {
                    if let AreaOrLineSymbol::Area(area_symbol) = p
                        && area_symbol.minimum_area.get() > 0.
                    {
                        min = min.min(area_symbol.minimum_area.get());
                    }
                }
            }
        }
        if min == f64::MAX {
            return Ok(0.);
        }
        Ok(min)
    }

    /// Check if this symbol definition is cyclic.
    ///
    /// Uses an explicit visited set to detect cycles reliably.
    pub(super) fn contains_cycle(&self) -> Result<bool> {
        let mut visited_area = HashSet::new();
        let mut visited_line = HashSet::new();
        self.contains_cycle_with_visited(&mut visited_area, &mut visited_line)
    }

    fn contains_cycle_with_visited(
        &self,
        visited_area: &mut HashSet<*const RefCell<CombinedAreaSymbol>>,
        visited_line: &mut HashSet<*const RefCell<CombinedLineSymbol>>,
    ) -> Result<bool> {
        for part in &self.parts {
            match part {
                PublicOrPrivateSymbol::Public(WeakPathSymbol::CombinedArea(weak)) => {
                    if let Some(ca) = weak.upgrade() {
                        let ptr = std::rc::Rc::as_ptr(&ca);
                        if !visited_area.insert(ptr) {
                            return Ok(true); // Already visited — cycle detected
                        }
                        let borrowed = ca.try_borrow().map_err(|_| Error::SymbolCycleBorrow)?;
                        if borrowed.contains_cycle_with_visited(visited_area, visited_line)? {
                            return Ok(true);
                        }
                        let _ = visited_area.remove(&ptr);
                    }
                }
                PublicOrPrivateSymbol::Public(WeakPathSymbol::CombinedLine(weak)) => {
                    if let Some(cl) = weak.upgrade() {
                        let ptr = std::rc::Rc::as_ptr(&cl);
                        if !visited_line.insert(ptr) {
                            return Ok(true); // Already visited — cycle detected
                        }
                        let borrowed = cl.try_borrow().map_err(|_| Error::SymbolCycleBorrow)?;
                        if borrowed.contains_cycle_line_with_visited(visited_line)? {
                            return Ok(true);
                        }
                        let _ = visited_line.remove(&ptr);
                    }
                }
                _ => (),
            }
        }
        Ok(false)
    }

    // This will recurse forever if any cycles exist,
    // but it should not as the components are private and the addition of components are shielded
    /// Check if the symbol references the other symbol
    /// The check fails if any sub-symbol cannot be borrowed (is mutably borrowed somewhere else)
    pub fn contains_symbol(&self, other_symbol: &WeakSymbol) -> Result<bool> {
        match other_symbol {
            WeakSymbol::Point(_) | WeakSymbol::Text(_) => return Ok(false),
            _ => (),
        }

        for part in &self.parts {
            if let PublicOrPrivateSymbol::Public(s) = part {
                match (s, other_symbol) {
                    (WeakPathSymbol::CombinedArea(weak), _) => {
                        let combined_area = weak.upgrade();
                        if let Some(ca) = combined_area
                            && ca.try_borrow()?.contains_symbol(other_symbol)?
                        {
                            return Ok(true);
                        }
                    }
                    (WeakPathSymbol::CombinedLine(weak), _) => {
                        let combined_line = weak.upgrade();
                        if let Some(cl) = combined_line
                            && cl.try_borrow()?.contains_symbol(other_symbol)?
                        {
                            return Ok(true);
                        }
                    }
                    (WeakPathSymbol::Area(weak), WeakSymbol::Area(other_weak)) => {
                        if weak.ptr_eq(other_weak) {
                            return Ok(true);
                        }
                    }
                    (WeakPathSymbol::Line(weak), WeakSymbol::Line(other_weak)) => {
                        if weak.ptr_eq(other_weak) {
                            return Ok(true);
                        }
                    }
                    _ => (),
                }
            }
        }
        Ok(false)
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<(CombinedAreaSymbol, Vec<usize>)> {
        let mut common = attributes;
        let mut parts: Vec<PublicOrPrivateSymbol<WeakPathSymbol, AreaOrLineSymbol>> = Vec::new();
        let mut public_component_ids: Vec<usize> = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"description" => {
                        if let Event::Text(text) = reader.read_event_into(&mut buf)? {
                            common.description = String::from_utf8(text.to_vec())?;
                        }
                    }
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
                Event::End(e) => {
                    if e.local_name().as_ref() == b"symbol" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::CombinedAreaSymbol));
                }
                _ => {}
            }
        }

        Ok((CombinedAreaSymbol { common, parts }, public_component_ids))
    }

    fn parse_private_part<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
    ) -> Result<AreaOrLineSymbol> {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => {
                    if e.local_name().as_ref() == b"symbol" {
                        let sym_type: u8 = try_get_attr_raw(&e, "type")?.unwrap_or(0);
                        let mut sub_common = SymbolCommon::default();
                        for attr in e.attributes().filter_map(std::result::Result::ok) {
                            match attr.key.local_name().as_ref() {
                                b"name" => {
                                    sub_common.name =
                                        parse_attr(attr, e.decoder()).unwrap_or(sub_common.name);
                                }
                                b"code" => {
                                    sub_common.code = crate::utils::parse_attr_raw(attr.value)
                                        .unwrap_or_default();
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
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == b"part" {
                        return Err(Error::EmptyPrivatePart);
                    }
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
                Event::End(e) => {
                    if e.local_name().as_ref() == b"part" {
                        return Ok(());
                    }
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
            (
                "name",
                quick_xml::escape::escape(self.common.name.as_str()).as_ref(),
            ),
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
                PublicOrPrivateSymbol::Public(weak_path) => {
                    let sym_index = if let Some(sym) = weak_path.upgrade() {
                        symbol_set
                            .iter()
                            .position(|s| s == &sym)
                            .map(|p| p as i32)
                            .unwrap_or(-1)
                    } else {
                        -1
                    };

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
                            line.write(writer, color_set, None)?;
                        }
                        AreaOrLineSymbol::Area(area) => {
                            area.write(writer, color_set, None)?;
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
