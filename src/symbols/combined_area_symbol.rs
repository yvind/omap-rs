use quick_xml::{Reader, events::Event};

use super::{AreaSymbol, CombinedAreaSymbol, LineSymbol, PublicOrPrivateSymbol, SymbolCommon};
use crate::{
    Error, OmapSection, Result,
    colors::{ColorId, ColorSet},
    notes,
    symbols::{AreaOrLineSymbol, PathSymbolId, Symbol, SymbolId, SymbolSet},
    utils::{parse_attr, parse_attr_raw, try_get_attr_raw},
};

impl CombinedAreaSymbol {
    /// Get the minimum area (in paper dimensions mm²) among all area sub-symbols.
    ///
    /// Takes the [`SymbolSet`] that owns the public components. A component no
    /// longer in the set contributes nothing.
    pub fn minimum_area(&self, symbol_set: &SymbolSet) -> f64 {
        let mut min = f64::MAX;
        for part in self.components() {
            let area = match part {
                PublicOrPrivateSymbol::Public(id) => match symbol_set.get(SymbolId::from(*id)) {
                    Some(Symbol::Area(symbol)) => symbol.minimum_area.get(),
                    Some(Symbol::CombinedArea(symbol)) => symbol.minimum_area(symbol_set),
                    _ => 0.,
                },
                PublicOrPrivateSymbol::Private(AreaOrLineSymbol::Area(symbol)) => {
                    symbol.minimum_area.get()
                }
                PublicOrPrivateSymbol::Private(AreaOrLineSymbol::Line(_)) => 0.,
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
        self.component_colors(symbol_set)
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
                            let sym = Self::parse_private_part(reader, color_set)?;
                            parts.push(PublicOrPrivateSymbol::Private(sym));
                        } else {
                            let symbol_index = try_get_attr_raw::<i32>(&e, "symbol")?;
                            // Mapper writes -1 for unknown or empty public parts.
                            if let Some(symbol_index) = symbol_index.filter(|&id| id >= 0) {
                                public_component_ids.push(symbol_index as usize);
                            }
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

        Ok((Self::from_parts(common, parts), public_component_ids))
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
                                sub_common.code = parse_attr_raw(attr.value).unwrap_or_default();
                            }
                            _ => {}
                        }
                    }
                    match sym_type {
                        2 => {
                            let line = LineSymbol::parse(reader, color_set, sub_common)?;
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
}
