use std::{cell::RefCell, fmt::Debug, rc::Weak};

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{AreaSymbol, LineSymbol, PubOrPrivSymbol, Symbol, SymbolCommon, SymbolSet};
use crate::{Error, Result, colors::ColorSet, utils::try_get_attr};

/// A combined area symbol composed of multiple sub-symbols.
#[derive(Debug, Clone)]
pub struct CombinedAreaSymbol {
    /// Common symbol properties.
    pub common: SymbolCommon,
    /// The component parts of this combined symbol.
    pub parts: Vec<PubOrPrivSymbol<WeakPathSymbol, PathSymbol>>,
}

/// An area or line sub-symbol used in a combined symbol (private variant).
#[derive(Debug, Clone)]
pub enum PathSymbol {
    /// An area sub-symbol.
    Area(Box<AreaSymbol>),
    /// A line sub-symbol.
    Line(Box<LineSymbol>),
}

/// A non-owning reference to a path sub-symbol.
#[derive(Debug, Clone)]
pub enum WeakPathSymbol {
    /// A weak reference to an area symbol.
    Area(Weak<RefCell<AreaSymbol>>),
    /// A weak reference to a line symbol.
    Line(Weak<RefCell<LineSymbol>>),
}

impl WeakPathSymbol {
    /// Attempt to upgrade the weak reference to a strong [`Symbol`].
    pub fn upgrade(&self) -> Option<Symbol> {
        match self {
            WeakPathSymbol::Area(weak) => weak.upgrade().map(Symbol::Area),
            WeakPathSymbol::Line(weak) => weak.upgrade().map(Symbol::Line),
        }
    }
}

impl CombinedAreaSymbol {
    /// Get the display name of this combined area symbol.
    pub fn get_name(&self) -> &str {
        &self.common.name
    }

    /// Get the minimum area (in mm²) among all area sub-symbols.
    pub fn minimum_area(&self) -> Result<f64> {
        let mut min = f64::MAX;
        for s in self.parts.iter() {
            match s {
                PubOrPrivSymbol::Public(p) => {
                    if let WeakPathSymbol::Area(weak) = p
                        && let Some(area) = weak.upgrade()
                    {
                        let area_symbol = area.try_borrow()?;
                        if area_symbol.minimum_area.get() > 0. {
                            min = min.min(area_symbol.minimum_area.get());
                        }
                    }
                }
                PubOrPrivSymbol::Private(p) => {
                    if let PathSymbol::Area(area_symbol) = p
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

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<(CombinedAreaSymbol, Vec<usize>)> {
        let mut common = attributes;
        let mut parts: Vec<PubOrPrivSymbol<WeakPathSymbol, PathSymbol>> = Vec::new();
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
                        let is_private = try_get_attr(&e, "private").unwrap_or(false);
                        if is_private {
                            // Parse the private sub-symbol
                            let sym = Self::parse_private_part(reader, color_set)?;
                            parts.push(PubOrPrivSymbol::Private(sym));
                        } else {
                            let symbol_index = try_get_attr(&e, "symbol").unwrap_or(usize::MAX);
                            // Record the public component ID for later resolution
                            public_component_ids.push(symbol_index);
                            // Don't push to parts here - will be resolved by symbol_set after all symbols are loaded
                        }
                    }
                    _ => {}
                },
                Event::Empty(e) => {
                    if e.local_name().as_ref() == b"icon" {
                        if let Some(src) = try_get_attr::<String>(&e, "src") {
                            common.custom_icon = Some(src);
                        }
                    } else if e.local_name().as_ref() == b"part" {
                        let is_private = try_get_attr(&e, "private").unwrap_or(false);
                        if !is_private {
                            let symbol_index = try_get_attr(&e, "symbol").unwrap_or(usize::MAX);
                            public_component_ids.push(symbol_index);
                        }
                    }
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == b"symbol" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in CombinedAreaSymbol parsing".to_string(),
                    ));
                }
                _ => {}
            }
        }

        Ok((CombinedAreaSymbol { common, parts }, public_component_ids))
    }

    fn parse_private_part<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
    ) -> Result<PathSymbol> {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => {
                    if e.local_name().as_ref() == b"symbol" {
                        let sym_type: u8 = try_get_attr(&e, "type").unwrap_or(0);
                        let mut sub_common = SymbolCommon::default();
                        for attr in e.attributes().filter_map(std::result::Result::ok) {
                            match attr.key.local_name().as_ref() {
                                b"name" => {
                                    sub_common.name.push_str(&quick_xml::escape::unescape(
                                        std::str::from_utf8(&attr.value)?,
                                    )?);
                                }
                                b"code" => {
                                    sub_common.code =
                                        crate::utils::parse_attr(attr.value).unwrap_or_default();
                                }
                                _ => {}
                            }
                        }
                        match sym_type {
                            2 => {
                                let line = LineSymbol::parse(reader, color_set, sub_common)?;
                                // Skip to end of part
                                Self::skip_to_end_of_part(reader)?;
                                return Ok(PathSymbol::Line(Box::new(line)));
                            }
                            4 => {
                                let area = AreaSymbol::parse(reader, color_set, sub_common)?;
                                Self::skip_to_end_of_part(reader)?;
                                return Ok(PathSymbol::Area(Box::new(area)));
                            }
                            _ => {
                                return Err(Error::ParseOmapFileError(format!(
                                    "Unknown private part symbol type {sym_type}"
                                )));
                            }
                        }
                    }
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == b"part" {
                        return Err(Error::ParseOmapFileError("Empty private part".to_string()));
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in private part parsing".to_string(),
                    ));
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
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF skipping to end of part".to_string(),
                    ));
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
                quick_xml::escape::unescape(self.common.name.as_str())?.as_ref(),
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
                PubOrPrivSymbol::Public(weak_path) => {
                    let sym = weak_path.upgrade().ok_or(Error::SymbolError)?;
                    let sym_index = symbol_set
                        .iter()
                        .enumerate()
                        .find(|(_, s)| *s == &sym)
                        .map(|(i, _)| i)
                        .ok_or(Error::SymbolError)?;
                    writer.write_event(Event::Empty(
                        BytesStart::new("part")
                            .with_attributes([("symbol", sym_index.to_string().as_str())]),
                    ))?;
                }
                PubOrPrivSymbol::Private(path_sym) => {
                    writer.write_event(Event::Start(
                        BytesStart::new("part").with_attributes([("private", "true")]),
                    ))?;
                    match path_sym {
                        PathSymbol::Line(line) => {
                            line.write(writer, color_set, None)?;
                        }
                        PathSymbol::Area(area) => {
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
