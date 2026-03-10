use std::{cell::RefCell, rc::Weak};

use quick_xml::{
    Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{LineSymbol, PubOrPrivSymbol, Symbol, SymbolCommon, SymbolSet};
use crate::{Error, Result, colors::ColorSet};

/// A combined line symbol composed of multiple sub-symbols.
#[derive(Debug, Clone)]
pub struct CombinedLineSymbol {
    /// Common symbol properties.
    pub common: SymbolCommon,
    /// The component parts of this combined symbol.
    pub parts: Vec<PubOrPrivSymbol<Weak<RefCell<LineSymbol>>, Box<LineSymbol>>>,
}

impl CombinedLineSymbol {
    /// Get the display name of this combined line symbol.
    pub fn get_name(&self) -> &str {
        &self.common.name
    }

    /// Get the minimum length (in mm) among all line sub-symbols.
    pub fn minimum_length(&self) -> Result<f64> {
        let mut min = f64::MAX;
        for s in self.parts.iter() {
            match s {
                PubOrPrivSymbol::Public(weak) => {
                    if let Some(line) = weak.upgrade() {
                        let line_symbol = line.try_borrow()?;
                        if line_symbol.minimum_length.get() > 0. {
                            min = min.min(line_symbol.minimum_length.get());
                        }
                    }
                }
                PubOrPrivSymbol::Private(line_symbol) => {
                    if line_symbol.minimum_length.get() > 0. {
                        min = min.min(line_symbol.minimum_length.get());
                    }
                }
            }
        }
        if min == f64::MAX {
            return Ok(0.);
        }
        Ok(min)
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
                PubOrPrivSymbol::Public(weak) => {
                    let rc = weak.upgrade().ok_or(Error::SymbolError)?;
                    let sym = Symbol::Line(rc);
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
                PubOrPrivSymbol::Private(line) => {
                    writer.write_event(Event::Start(
                        BytesStart::new("part").with_attributes([("private", "true")]),
                    ))?;
                    line.write(writer, color_set, None)?;
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
