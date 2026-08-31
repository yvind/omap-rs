use quick_xml::{
    Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{
    AreaOrLineSymbol, LineSymbol, PublicOrPrivateSymbol, SymbolCommon, SymbolSet,
    symbol::SymbolPosition,
};
use crate::{
    Code, Result,
    colors::{ColorId, ColorSet},
    symbols::{LinePathSymbolId, PathSymbolId, Symbol, SymbolId},
};

pub(crate) trait PrivatePart {
    fn colors(&self) -> Vec<ColorId>;

    fn write_part<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
    ) -> Result<()>;
}

impl PrivatePart for AreaOrLineSymbol {
    fn colors(&self) -> Vec<ColorId> {
        match self {
            Self::Area(symbol) => symbol.colors(),
            Self::Line(symbol) => symbol.colors(),
        }
    }

    fn write_part<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
    ) -> Result<()> {
        match self {
            Self::Area(symbol) => symbol.write(writer, color_set, SymbolPosition::Private),
            Self::Line(symbol) => symbol.write(writer, color_set, SymbolPosition::Private),
        }
    }
}

impl PrivatePart for LineSymbol {
    fn colors(&self) -> Vec<ColorId> {
        Self::colors(self)
    }

    fn write_part<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
    ) -> Result<()> {
        self.write(writer, color_set, SymbolPosition::Private)
    }
}

/// A symbol built from other symbols, public or private.
///
/// `Id` is the handle a public component may use, `Private` the symbol a
/// private component may be.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CombinedSymbol<Id, Private> {
    /// Common symbol properties.
    pub common: SymbolCommon,
    /// The component parts of this combined symbol.
    /// Public components are added through [`SymbolSet::add_area_component`]
    /// and [`SymbolSet::add_line_component`], which reject any component that
    /// would make the definition cyclic.
    parts: Vec<PublicOrPrivateSymbol<Id, Private>>,
}

/// A combined area symbol: any path symbol public, an area or line private.
pub type CombinedAreaSymbol = CombinedSymbol<PathSymbolId, AreaOrLineSymbol>;

/// A combined line symbol: a line or combined line public, a line private.
pub type CombinedLineSymbol = CombinedSymbol<LinePathSymbolId, LineSymbol>;

impl<Id: Copy + Into<SymbolId>, Private> CombinedSymbol<Id, Private> {
    /// Create a new empty combined symbol with the given code and name.
    pub fn new(code: Code, name: impl Into<String>) -> Self {
        Self {
            common: SymbolCommon {
                code,
                name: name.into(),
                ..Default::default()
            },
            parts: Vec::new(),
        }
    }

    pub(super) fn from_parts(
        common: SymbolCommon,
        parts: Vec<PublicOrPrivateSymbol<Id, Private>>,
    ) -> Self {
        Self { common, parts }
    }

    /// Iterate through the symbol component of the symbol
    pub fn components(&self) -> impl Iterator<Item = &PublicOrPrivateSymbol<Id, Private>> {
        self.parts.iter()
    }

    /// Iterate over only the public components.
    pub fn public_components(&self) -> impl Iterator<Item = Id> {
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
    pub fn remove_component(&mut self, index: usize) -> Option<PublicOrPrivateSymbol<Id, Private>> {
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
    ) -> Option<PublicOrPrivateSymbol<Id, Private>> {
        if self.parts.len() > index {
            Some(self.parts.swap_remove(index))
        } else {
            None
        }
    }

    pub(crate) fn push_component(&mut self, component: PublicOrPrivateSymbol<Id, Private>) {
        self.parts.push(component);
    }

    /// Get the display name of this combined symbol.
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

    /// Does this symbol reference `other`, directly or through a component?
    ///
    /// Takes the [`SymbolSet`] that owns the public components.
    pub fn contains_symbol(&self, symbol_set: &SymbolSet, other: SymbolId) -> bool {
        self.public_components().any(|component| {
            let component = component.into();
            if component == other {
                return true;
            }
            match symbol_set.get(component).map(|symbol| symbol.symbol()) {
                Some(Symbol::CombinedArea(symbol)) => symbol.contains_symbol(symbol_set, other),
                Some(Symbol::CombinedLine(symbol)) => symbol.contains_symbol(symbol_set, other),
                _ => false,
            }
        })
    }

    pub(crate) fn retain_map_components<F>(&mut self, mut f: F)
    where
        F: FnMut(PublicOrPrivateSymbol<Id, Private>) -> Option<PublicOrPrivateSymbol<Id, Private>>,
    {
        self.parts = std::mem::take(&mut self.parts)
            .into_iter()
            .filter_map(&mut f)
            .collect();
    }

    pub(crate) fn component_colors(&self, symbol_set: &SymbolSet) -> Vec<ColorId>
    where
        Private: PrivatePart,
    {
        let mut colors = Vec::new();
        for component in self.components() {
            match component {
                PublicOrPrivateSymbol::Public(id) => {
                    if let Some(symbol) = symbol_set.get((*id).into()) {
                        colors.extend(symbol.colors(symbol_set));
                    }
                }
                PublicOrPrivateSymbol::Private(symbol) => colors.extend(symbol.colors()),
            }
        }
        colors
    }

    pub(super) fn write_body<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
        color_set: &ColorSet,
    ) -> Result<()>
    where
        Private: PrivatePart,
    {
        let mut cs = BytesStart::new("combined_symbol");
        cs.push_attribute(("parts", self.parts.len().to_string().as_str()));
        writer.write_event(Event::Start(cs))?;

        for part in &self.parts {
            match part {
                PublicOrPrivateSymbol::Public(id) => {
                    let sym_index = symbol_set.file_index(Some(*id));
                    writer.write_event(Event::Empty(
                        BytesStart::new("part")
                            .with_attributes([("symbol", sym_index.to_string().as_str())]),
                    ))?;
                }
                PublicOrPrivateSymbol::Private(symbol) => {
                    writer.write_event(Event::Start(
                        BytesStart::new("part").with_attributes([("private", "true")]),
                    ))?;
                    symbol.write_part(writer, color_set)?;
                    writer.write_event(Event::End(BytesEnd::new("part")))?;
                }
            }
        }

        writer.write_event(Event::End(BytesEnd::new("combined_symbol")))?;
        Ok(())
    }
}
