use std::collections::HashSet;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::Symbol;
use crate::arena::Arena;
use crate::{
    Code, Error, OmapSection, Result,
    colors::ColorSet,
    symbols::{
        AreaOrLineSymbol, AreaSymbol, CombinedAreaSymbol, CombinedAreaSymbolId, CombinedLineSymbol,
        CombinedLineSymbolId, LinePathSymbolId, LineSymbol, PathSymbolId, PointSymbol,
        PublicOrPrivateSymbol, SymbolId, TextSymbol,
    },
    utils::{try_get_attr, try_get_attr_raw},
};

/// A collection of symbols.
///
/// Position in the set is the symbol's file index, which is what the `.omap`
/// format stores in every object and every combined-symbol component. A
/// [`SymbolId`] is independent of that: it keeps naming the same symbol across
/// [`SymbolSet::sort`], and stops resolving once that symbol is removed.
///
/// A [`SymbolId`] left over from a removed symbol is written as the format's
/// `-1` unknown-symbol sentinel, exactly as a dangling weak reference was.
#[derive(Debug, Clone)]
pub struct SymbolSet {
    symbols: Arena<Symbol>,
    /// The name of the symbol set.
    pub name: String,
}

macro_rules! impl_typed_accessors {
    ($get:ident, $get_mut:ident, $iter:ident, $id:ident, $symbol:ident, $variant:ident) => {
        /// Get a symbol of this kind by its handle. Cannot return another kind.
        pub fn $get(&self, id: crate::symbols::$id) -> Option<&$symbol> {
            match self.symbols.get(id.0) {
                Some(Symbol::$variant(symbol)) => Some(symbol),
                _ => None,
            }
        }

        /// Mutably get a symbol of this kind by its handle.
        pub fn $get_mut(&mut self, id: crate::symbols::$id) -> Option<&mut $symbol> {
            match self.symbols.get_mut(id.0) {
                Some(Symbol::$variant(symbol)) => Some(symbol),
                _ => None,
            }
        }

        /// Iterate over only the symbols of this kind, with their handles.
        pub fn $iter(&self) -> impl Iterator<Item = (crate::symbols::$id, &$symbol)> {
            self.symbols
                .iter()
                .filter_map(|(raw, symbol)| match symbol {
                    Symbol::$variant(symbol) => Some((crate::symbols::$id(raw), symbol)),
                    _ => None,
                })
        }
    };
}

impl SymbolSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            symbols: Arena::new(),
            name: name.into(),
        }
    }

    /// Get the number of symbols in the [`SymbolSet`].
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns `true` if the symbol set contains no symbols.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Returns `true` if `id` names a symbol still in this set.
    pub fn contains(&self, id: SymbolId) -> bool {
        self.symbols.contains(id.raw())
    }

    /// Add a new symbol to the [`SymbolSet`]
    pub fn add_symbol(&mut self, symbol: impl Into<Symbol>) -> SymbolId {
        let symbol = symbol.into();
        let raw = self.symbols.push(symbol);
        #[expect(
            clippy::unwrap_used,
            reason = "the handle was just returned by the push that created it"
        )]
        self.symbols.get(raw).unwrap().id_for(raw)
    }

    /// Get a symbol by its handle.
    pub fn get(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.raw())
    }

    /// Mutably get a symbol by its handle.
    pub fn get_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(id.raw())
    }

    impl_typed_accessors!(
        point_symbol,
        point_symbol_mut,
        iter_point_symbols,
        PointSymbolId,
        PointSymbol,
        Point
    );
    impl_typed_accessors!(
        line_symbol,
        line_symbol_mut,
        iter_line_symbols,
        LineSymbolId,
        LineSymbol,
        Line
    );
    impl_typed_accessors!(
        area_symbol,
        area_symbol_mut,
        iter_area_symbols,
        AreaSymbolId,
        AreaSymbol,
        Area
    );
    impl_typed_accessors!(
        text_symbol,
        text_symbol_mut,
        iter_text_symbols,
        TextSymbolId,
        TextSymbol,
        Text
    );
    impl_typed_accessors!(
        combined_area_symbol,
        combined_area_symbol_mut,
        iter_combined_area_symbols,
        CombinedAreaSymbolId,
        CombinedAreaSymbol,
        CombinedArea
    );
    impl_typed_accessors!(
        combined_line_symbol,
        combined_line_symbol_mut,
        iter_combined_line_symbols,
        CombinedLineSymbolId,
        CombinedLineSymbol,
        CombinedLine
    );

    /// Remove and return the [`Symbol`] the handle names.
    pub fn remove(&mut self, id: SymbolId) -> Option<Symbol> {
        self.symbols.remove(id.raw())
    }

    /// Remove and return the first [`Symbol`] with the given [`Code`].
    pub fn remove_by_code(&mut self, code: Code) -> Option<Symbol> {
        let index = self.values().position(|s| s.common().code == code)?;
        self.symbols.remove_at(index)
    }

    /// Remove and return the first [`Symbol`] with the given name.
    pub fn remove_by_name(&mut self, name: &str) -> Option<Symbol> {
        let index = self.values().position(|s| s.common().name == name)?;
        self.symbols.remove_at(index)
    }

    /// Find a symbol by its [Code]. The first match is returned.
    pub fn symbol_by_code(&self, code: Code) -> Option<&Symbol> {
        self.values().find(|s| s.common().code == code)
    }

    /// Find a handle to a symbol by its [Code]. The first match is returned.
    pub fn id_by_code(&self, code: Code) -> Option<SymbolId> {
        self.iter()
            .find(|(_, s)| s.common().code == code)
            .map(|(id, _)| id)
    }

    /// Find a [Symbol] by its display name. The first match is returned.
    pub fn symbol_by_name(&self, name: &str) -> Option<&Symbol> {
        self.values().find(|s| s.common().name == name)
    }

    /// Find a handle to a [Symbol] by its display name. The first match is returned.
    pub fn id_by_name(&self, name: &str) -> Option<SymbolId> {
        self.iter()
            .find(|(_, s)| s.common().name == name)
            .map(|(id, _)| id)
    }

    /// Get the symbol at a file index.
    pub fn symbol_at(&self, index: usize) -> Option<&Symbol> {
        self.symbols.get_at(index)
    }

    /// Get a handle to the symbol at a file index.
    pub fn id_at(&self, index: usize) -> Option<SymbolId> {
        let symbol = self.symbols.get_at(index)?;
        Some(symbol.id_for(self.symbols.id_at(index)?))
    }

    /// Get the file index of a symbol, or `None` if it is not in this set.
    ///
    /// This is the integer the `.omap` format stores for every reference to the
    /// symbol, and the lookup every write performs.
    pub fn index_of(&self, id: SymbolId) -> Option<usize> {
        self.symbols.position(id.raw())
    }

    /// Iterate over the symbols and their handles, in file order.
    pub fn iter(&self) -> impl Iterator<Item = (SymbolId, &Symbol)> {
        self.symbols
            .iter()
            .map(|(raw, symbol)| (symbol.id_for(raw), symbol))
    }

    /// Iterate over the symbols in file order.
    pub fn values(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }

    /// Mutably iterate over the symbols in file order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Symbol> {
        self.symbols.values_mut()
    }

    /// Iterate over handles to every symbol, in file order.
    pub fn ids(&self) -> impl Iterator<Item = SymbolId> {
        self.iter().map(|(id, _)| id)
    }

    /// Sort the symbols by [`Code`]. Every handle stays valid.
    pub fn sort(&mut self) {
        self.symbols.sort_by_key(|symbol| symbol.common().code);
    }
}

impl SymbolSet {
    /// Add a component to a combined area symbol.
    ///
    /// Cycle detection has to walk the whole set, which a `&mut` borrow of a
    /// single symbol cannot do, so the checked mutation lives here rather than
    /// on [`CombinedAreaSymbol`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::CyclicSymbolDefinition`] if the component would make
    /// the definition cyclic, or [`Error::SymbolConversionError`] if `target`
    /// does not name a combined area symbol in this set.
    pub fn add_area_component(
        &mut self,
        target: CombinedAreaSymbolId,
        component: PublicOrPrivateSymbol<PathSymbolId, AreaOrLineSymbol>,
    ) -> Result<()> {
        if let PublicOrPrivateSymbol::Public(public) = &component
            && self.would_cycle(SymbolId::CombinedArea(target), *public)
        {
            return Err(Error::CyclicSymbolDefinition);
        }
        self.combined_area_symbol_mut(target)
            .ok_or(Error::SymbolConversionError)?
            .push_component(component);
        Ok(())
    }

    /// Add a component to a combined line symbol.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CyclicSymbolDefinition`] if the component would make
    /// the definition cyclic, or [`Error::SymbolConversionError`] if `target`
    /// does not name a combined line symbol in this set.
    pub fn add_line_component(
        &mut self,
        target: CombinedLineSymbolId,
        component: PublicOrPrivateSymbol<LinePathSymbolId, Box<LineSymbol>>,
    ) -> Result<()> {
        if let PublicOrPrivateSymbol::Public(public) = &component
            && self.would_cycle(SymbolId::CombinedLine(target), (*public).into())
        {
            return Err(Error::CyclicSymbolDefinition);
        }
        self.combined_line_symbol_mut(target)
            .ok_or(Error::SymbolConversionError)?
            .push_component(component);
        Ok(())
    }

    /// Would adding `component` to `target` make the combined symbol
    /// definitions cyclic?
    ///
    /// One visited set, no raw pointers, no borrows, and no way to fail.
    pub fn would_cycle(&self, target: SymbolId, component: PathSymbolId) -> bool {
        let mut visited = HashSet::new();
        self.reaches(SymbolId::from(component), target, &mut visited)
    }

    fn reaches(&self, from: SymbolId, goal: SymbolId, visited: &mut HashSet<SymbolId>) -> bool {
        if from == goal {
            return true;
        }
        if !visited.insert(from) {
            return false;
        }
        let components: Vec<SymbolId> = match self.get(from) {
            Some(Symbol::CombinedArea(symbol)) => {
                symbol.public_components().map(Into::into).collect()
            }
            Some(Symbol::CombinedLine(symbol)) => {
                symbol.public_components().map(Into::into).collect()
            }
            _ => return false,
        };
        components
            .into_iter()
            .any(|next| self.reaches(next, goal, visited))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "symbol-set parsing also resolves combined-symbol references"
    )]
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        colors: &ColorSet,
    ) -> Result<Self> {
        let symbol_set_name = try_get_attr(element, "id")?.unwrap_or_else(|| "Custom".to_owned());
        let count = try_get_attr_raw(element, "count")
            .ok()
            .flatten()
            .unwrap_or(1);

        let mut symbols = vec![None; count];
        let mut components = vec![Vec::new(); count];

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"symbol") {
                        let (symbol_id, symbol, combined_components) =
                            Symbol::parse(reader, &bytes_start, colors)?;
                        if symbol_id >= symbols.len() {
                            return Err(Error::SymbolIdOutOfRange(symbol_id));
                        }
                        if symbols[symbol_id].is_some() {
                            return Err(Error::DuplicateSymbolId(symbol_id));
                        }
                        components[symbol_id] = combined_components;
                        symbols[symbol_id] = Some(symbol);
                    }
                }
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"symbols") {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::Symbols));
                }
                _ => (),
            }
        }
        let mut symbols: Vec<Symbol> = symbols
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(Error::SymbolCountMismatch)?;

        // Before linking public components, identify CombinedArea symbols
        // that should actually be CombinedLine symbols.
        // At this point, only private parts are populated in the CombinedAreaSymbols.

        // Step 1: Initial candidates — CombinedArea symbols with no private Area parts
        // and whose public component IDs don't reference Area/Point/Text symbols.
        let mut candidate_indices: Vec<usize> = Vec::new();
        for (i, symbol) in symbols.iter().enumerate() {
            if let Symbol::CombinedArea(combined) = symbol {
                let has_private_area = combined.components().any(|p| {
                    matches!(p, PublicOrPrivateSymbol::Private(AreaOrLineSymbol::Area(_)))
                });
                if has_private_area {
                    continue;
                }
                let has_area_public = components[i].iter().any(|&id| {
                    matches!(
                        symbols.get(id),
                        // Point and text is not allowed in combined symbol, that is treated later on
                        Some(Symbol::Area(_) | Symbol::Point(_) | Symbol::Text(_))
                    )
                });
                if has_area_public {
                    continue;
                }
                candidate_indices.push(i);
            }
        }

        // Step 2: Iteratively remove candidates that reference CombinedArea symbols
        // that aren't themselves candidates (those are true area symbols).
        // A candidate referencing another candidate's CombinedArea is fine — both will be converted.
        loop {
            let prev_len = candidate_indices.len();
            let current_candidates = candidate_indices.clone();
            candidate_indices.retain(|&idx| {
                !components[idx].iter().any(|&id| {
                    matches!(symbols.get(id), Some(Symbol::CombinedArea(_)))
                        && !current_candidates.contains(&id)
                })
            });
            if candidate_indices.len() == prev_len {
                break;
            }
        }

        // Step 3: Convert candidates from CombinedArea to CombinedLine.
        // Only private parts need to be moved; public parts will be linked in Step 4.
        for &idx in &candidate_indices {
            let Symbol::CombinedArea(combined) = &mut symbols[idx] else {
                continue;
            };
            let mut converted = CombinedLineSymbol::new(Code::default(), String::new());
            converted.common = std::mem::take(&mut combined.common);
            let part_count = combined.components().count();
            for _ in 0..part_count {
                if let Some(PublicOrPrivateSymbol::Private(AreaOrLineSymbol::Line(line))) =
                    combined.remove_component(0)
                {
                    converted.push_component(PublicOrPrivateSymbol::Private(line));
                }
            }
            symbols[idx] = Symbol::CombinedLine(converted);
        }

        let mut symbol_set = Self {
            symbols: symbols.into_iter().collect(),
            name: symbol_set_name,
        };

        // Step 4: Link public components for all combined symbols.
        // This runs after conversion so the handles name the correct types.
        for (index, component_ids) in components.into_iter().enumerate() {
            if component_ids.is_empty() {
                continue;
            }
            let Some(target) = symbol_set.id_at(index) else {
                continue;
            };
            for id in component_ids {
                let component = symbol_set
                    .id_at(id)
                    .ok_or(Error::SymbolSetIndexOutOfRange(id))?;
                match target {
                    SymbolId::CombinedArea(target) => {
                        let component = PathSymbolId::try_from(component)
                            .map_err(|_| Error::CombinedSymbolContainsPointOrText)?;
                        symbol_set
                            .add_area_component(target, PublicOrPrivateSymbol::Public(component))?;
                    }
                    SymbolId::CombinedLine(target) => {
                        let component = LinePathSymbolId::try_from(component)
                            .map_err(|_| Error::CombinedLineSymbolContainsNonLine)?;
                        symbol_set
                            .add_line_component(target, PublicOrPrivateSymbol::Public(component))?;
                    }
                    _ => return Err(Error::ComponentsInNonCombinedSymbol),
                }
            }
        }

        Ok(symbol_set)
    }

    pub(crate) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        colors: &ColorSet,
    ) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("symbols").with_attributes([
            ("count", self.len().to_string().as_str()),
            ("id", self.name.as_str()),
        ])))?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        for (index, symbol) in self.values().enumerate() {
            symbol.write(writer, self, colors, index)?;
            writer.get_mut().write_all(b"\n".as_slice())?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbols")))?;
        Ok(())
    }
}
