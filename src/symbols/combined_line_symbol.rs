use quick_xml::{
    Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{LineSymbol, PublicOrPrivateSymbol, SymbolCommon, SymbolSet};
use crate::{
    Code, Result,
    colors::{ColorId, ColorSet},
    symbols::{LinePathSymbolId, Symbol, SymbolId},
};

/// A combined line symbol composed of multiple sub-symbols.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CombinedLineSymbol {
    /// Common symbol properties.
    pub common: SymbolCommon,
    /// Public components are added through [`SymbolSet::add_line_component`],
    /// which rejects any component that would make the definition cyclic.
    parts: Vec<PublicOrPrivateSymbol<LinePathSymbolId, Box<LineSymbol>>>,
}

impl CombinedLineSymbol {
    /// Iterate through the symbol component of the symbol
    pub fn components(
        &self,
    ) -> impl Iterator<Item = &PublicOrPrivateSymbol<LinePathSymbolId, Box<LineSymbol>>> {
        self.parts.iter()
    }

    /// Iterate over only the public components.
    pub fn public_components(&self) -> impl Iterator<Item = LinePathSymbolId> {
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
    ) -> Option<PublicOrPrivateSymbol<LinePathSymbolId, Box<LineSymbol>>> {
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
    ) -> Option<PublicOrPrivateSymbol<LinePathSymbolId, Box<LineSymbol>>> {
        if self.parts.len() > index {
            Some(self.parts.swap_remove(index))
        } else {
            None
        }
    }

    pub(crate) fn push_component(
        &mut self,
        component: PublicOrPrivateSymbol<LinePathSymbolId, Box<LineSymbol>>,
    ) {
        self.parts.push(component);
    }

    /// Create a new empty combined line symbol with the given code and name.
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

    /// Get the display name of this combined line symbol.
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

    /// Get the minimum length (in paper dimensions mm) among all line sub-symbols.
    ///
    /// Takes the [`SymbolSet`] that owns the public components. A component no
    /// longer in the set contributes nothing.
    pub fn minimum_length(&self, symbol_set: &SymbolSet) -> f64 {
        let mut min = f64::MAX;
        for part in &self.parts {
            let length = match part {
                PublicOrPrivateSymbol::Public(LinePathSymbolId::Line(id)) => symbol_set
                    .line_symbol(*id)
                    .map_or(0., |symbol| symbol.minimum_length.get()),
                PublicOrPrivateSymbol::Public(LinePathSymbolId::CombinedLine(id)) => symbol_set
                    .combined_line_symbol(*id)
                    .map_or(0., |symbol| symbol.minimum_length(symbol_set)),
                PublicOrPrivateSymbol::Private(symbol) => symbol.minimum_length.get(),
            };
            if length > 0. {
                min = min.min(length);
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
                PublicOrPrivateSymbol::Private(symbol) => colors.extend(symbol.colors()),
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
                Some(Symbol::CombinedLine(symbol)) => symbol.contains_symbol(symbol_set, other),
                _ => false,
            }
        })
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
                PublicOrPrivateSymbol::Private(line) => {
                    writer.write_event(Event::Start(
                        BytesStart::new("part").with_attributes([("private", "true")]),
                    ))?;
                    line.write(writer, color_set, None, false)?;
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

impl CombinedLineSymbol {
    pub(crate) fn retain_map_components<F>(&mut self, mut f: F)
    where
        F: FnMut(
            PublicOrPrivateSymbol<LinePathSymbolId, Box<LineSymbol>>,
        ) -> Option<PublicOrPrivateSymbol<LinePathSymbolId, Box<LineSymbol>>>,
    {
        self.parts = std::mem::take(&mut self.parts)
            .into_iter()
            .filter_map(&mut f)
            .collect();
    }
}
