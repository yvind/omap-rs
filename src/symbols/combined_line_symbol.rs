use quick_xml::{
    Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{LineSymbol, PubOrPrivSymbol, SymbolCommon, SymbolSet};
use crate::{
    Code, Error, Result,
    colors::ColorSet,
    symbols::{Symbol, WeakLinePathSymbol, WeakSymbol},
};

/// A combined line symbol composed of multiple sub-symbols.
#[derive(Debug, Clone)]
pub struct CombinedLineSymbol {
    /// Common symbol properties.
    pub common: SymbolCommon,
    parts: Vec<PubOrPrivSymbol<WeakLinePathSymbol, Box<LineSymbol>>>,
}

impl CombinedLineSymbol {
    /// Iterate through the symbol component of the symbol
    pub fn components(
        &self,
    ) -> impl Iterator<Item = &PubOrPrivSymbol<WeakLinePathSymbol, Box<LineSymbol>>> {
        self.parts.iter()
    }

    /// Iterate through the mutable symbol component of the symbol
    pub fn components_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut PubOrPrivSymbol<WeakLinePathSymbol, Box<LineSymbol>>> {
        self.parts.iter_mut()
    }

    /// Remove and return the symbol component at position `index` in the component vec.
    /// This preserves the order of the components. O(N) run time
    pub fn remove_component(
        &mut self,
        index: usize,
    ) -> Option<PubOrPrivSymbol<WeakLinePathSymbol, Box<LineSymbol>>> {
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
    ) -> Option<PubOrPrivSymbol<WeakLinePathSymbol, Box<LineSymbol>>> {
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
    /// are already being borrowed (through any of the .(try_)borrow(), .(try_)borrow_mut() functions) it fails
    pub fn add_component(
        &mut self,
        new_component: PubOrPrivSymbol<WeakLinePathSymbol, Box<LineSymbol>>,
    ) -> Result<()> {
        if matches!(
            new_component,
            PubOrPrivSymbol::Public(WeakLinePathSymbol::CombinedLine(_))
        ) {
            self.parts.push(new_component);
            match self.contains_cycle() {
                Ok(true) => {
                    let _ = self.parts.pop();
                    Err(Error::SymbolError(
                        "Adding this symbol would lead to a cyclic symbol defintion".to_string(),
                    ))
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

    /// Create a new empty combined line symbol with the given code and name.
    pub fn new(code: Code, name: String) -> CombinedLineSymbol {
        let common = SymbolCommon {
            code,
            name,
            ..Default::default()
        };
        CombinedLineSymbol {
            common,
            parts: Vec::new(),
        }
    }

    /// Get the display name of this combined line symbol.
    pub fn get_name(&self) -> &str {
        &self.common.name
    }

    /// Get the minimum length (in paper dimensions mm) among all line sub-symbols.
    /// The check fails if any child combined line symbols cannot be borrowed
    /// This will recurse forever if any cycles exist
    pub fn minimum_length(&self) -> Result<f64> {
        let mut min = f64::MAX;
        for s in self.parts.iter() {
            match s {
                PubOrPrivSymbol::Public(weak) => {
                    if let Some(line) = weak.upgrade() {
                        match line {
                            Symbol::Line(line) => {
                                let line_symbol = line.try_borrow()?;
                                if line_symbol.minimum_length.get() > 0. {
                                    min = min.min(line_symbol.minimum_length.get());
                                }
                            }
                            Symbol::CombinedLine(line) => {
                                let line_symbol = line.try_borrow()?;
                                let min_length = line_symbol.minimum_length()?;
                                if min_length > 0. {
                                    min = min.min(min_length);
                                }
                            }
                            _ => (),
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

    /// Check if this symbol definition is cyclic.
    ///
    /// This relies on the ref cells borrow checking
    pub(super) fn contains_cycle(&self) -> Result<bool> {
        for part in &self.parts {
            if let PubOrPrivSymbol::Public(WeakLinePathSymbol::CombinedLine(weak)) = part
                && let Some(cl) = weak.upgrade()
            {
                match cl.try_borrow_mut() {
                    Ok(borrowed) => {
                        if borrowed.contains_cycle()? {
                            return Ok(true);
                        }
                    }
                    Err(_) => return Ok(true), // Cannot borrow mut. Indicates a cycle
                }
            }
        }
        Ok(false)
    }

    // This will recurse forever if any cycles exist,
    // but it should not as the components are private and the addition of components are shielded
    /// Check if the symbol references the other symbol.
    /// The check fails if any sub-symbol cannot be borrowed (is mutably borrowed somewhere else)
    pub fn contains_symbol(&self, other_symbol: &WeakSymbol) -> Result<bool> {
        match other_symbol {
            WeakSymbol::Point(_)
            | WeakSymbol::Text(_)
            | WeakSymbol::Area(_)
            | WeakSymbol::CombinedArea(_) => return Ok(false),
            _ => (),
        }
        for part in &self.parts {
            if let PubOrPrivSymbol::Public(s) = part {
                match (s, other_symbol) {
                    (WeakLinePathSymbol::CombinedLine(weak), _) => {
                        let combined_line = weak.upgrade();
                        if let Some(cl) = combined_line
                            && cl.try_borrow()?.contains_symbol(other_symbol)?
                        {
                            return Ok(true);
                        }
                    }
                    (WeakLinePathSymbol::Line(weak), WeakSymbol::Line(other_weak)) => {
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
                PubOrPrivSymbol::Public(weak) => {
                    let sym_index = if let Some(sym) = weak.upgrade() {
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
