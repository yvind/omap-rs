use std::{cell::RefCell, rc::Rc};

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{Symbol, WeakSymbol};
use crate::{
    Code, Error, OmapSection, Result,
    colors::ColorSet,
    symbols::{
        AreaOrLineSymbol, AreaSymbol, CombinedAreaSymbol, CombinedLineSymbol, LineSymbol,
        PointSymbol, PublicOrPrivateSymbol, TextSymbol, WeakLinePathSymbol, WeakPathSymbol,
    },
    utils::{try_get_attr, try_get_attr_raw},
};

/// A collection of symbols.
#[derive(Debug)]
pub struct SymbolSet {
    /// The symbols in this set.
    symbols: Vec<Symbol>,
    /// The name of the symbol set.
    pub name: String,
}

impl SymbolSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            symbols: Vec::new(),
            name: name.into(),
        }
    }

    /// Get the number of symbols in the [`SymbolSet`].
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn contains_symbol(&self, symbol: &WeakSymbol) -> bool {
        self.iter_weak().find(|w| w == symbol).is_some()
    }

    /// Add a new symbol to the [`SymbolSet`]
    pub fn add_symbol(&mut self, symbol: impl Into<Symbol>) -> WeakSymbol {
        let symbol = symbol.into();
        let weak = symbol.downgrade();
        self.symbols.push(symbol);
        weak
    }

    /// Remove and return the [`Symbol`] with the given [`Code`] from the [`SymbolSet`]
    ///
    /// # Errors
    ///
    /// Returns and error if a symbol could not be borrowed as it is mutably borrowed somewhere else
    pub fn remove_by_code(&mut self, code: Code) -> Result<Option<Symbol>> {
        let mut remove = None;
        for (i, s) in self.symbols.iter().enumerate() {
            if s.common()?.code == code {
                remove = Some(i);
                break;
            }
        }
        if let Some(i) = remove {
            Ok(Some(self.symbols.swap_remove(i)))
        } else {
            Ok(None)
        }
    }

    /// Remove and return the [`Symbol`] with the given name from the [`SymbolSet`]
    ///
    /// # Errors
    ///
    /// Returns and error if a symbol could not be borrowed as it is mutably borrowed somewhere else
    pub fn remove_by_name(&mut self, name: &str) -> Result<Option<Symbol>> {
        let mut remove = None;
        for (i, s) in self.symbols.iter().enumerate() {
            if s.common()?.name == name {
                remove = Some(i);
                break;
            }
        }
        if let Some(i) = remove {
            Ok(Some(self.symbols.swap_remove(i)))
        } else {
            Ok(None)
        }
    }

    /// Remove and return the [`Symbol`] corresponding to the given [`WeakSymbol`] from the [`SymbolSet`]
    pub fn remove_by_weak(&mut self, weak: WeakSymbol) -> Option<Symbol> {
        let mut remove = None;
        for (i, s) in self.iter_weak().enumerate() {
            if s == weak {
                remove = Some(i);
                break;
            }
        }
        if let Some(i) = remove {
            Some(self.symbols.swap_remove(i))
        } else {
            None
        }
    }

    /// Find a symbol by its [Code]. The first match is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if a symbol cannot be borrowed for code checking (because it is mutably borrowed somewhere else)
    pub fn symbol_by_code(&self, code: Code) -> Result<Option<&Symbol>> {
        for s in &self.symbols {
            if s.code()? == code {
                return Ok(Some(s));
            }
        }
        Ok(None)
    }

    /// Find a [Symbol] by its display name. The first match is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if a symbol cannot be borrowed for name checking (because it is mutably borrowed somewhere else)
    pub fn symbol_by_name(&self, name: &str) -> Result<Option<&Symbol>> {
        for s in &self.symbols {
            if s.common()?.name == name {
                return Ok(Some(s));
            }
        }
        Ok(None)
    }

    /// Iterate over non-owning references to all symbols.
    pub fn iter_weak(&self) -> impl Iterator<Item = WeakSymbol> {
        self.symbols.iter().map(|s| s.downgrade())
    }

    /// Access the symbols through an iterator
    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    /// Iterate over only the point symbols.
    pub fn iter_point_symbols(&self) -> impl Iterator<Item = &Rc<RefCell<PointSymbol>>> {
        self.symbols.iter().filter_map(|s| match s {
            Symbol::Point(ref_cell) => Some(ref_cell),
            _ => None,
        })
    }

    /// Iterate over only the line symbols.
    pub fn iter_line_symbols(&self) -> impl Iterator<Item = &Rc<RefCell<LineSymbol>>> {
        self.symbols.iter().filter_map(|s| match s {
            Symbol::Line(ref_cell) => Some(ref_cell),
            _ => None,
        })
    }

    /// Iterate over only the area symbols.
    pub fn iter_area_symbols(&self) -> impl Iterator<Item = &Rc<RefCell<AreaSymbol>>> {
        self.symbols.iter().filter_map(|s| match s {
            Symbol::Area(ref_cell) => Some(ref_cell),
            _ => None,
        })
    }

    /// Iterate over only the text symbols.
    pub fn iter_text_symbols(&self) -> impl Iterator<Item = &Rc<RefCell<TextSymbol>>> {
        self.symbols.iter().filter_map(|s| match s {
            Symbol::Text(ref_cell) => Some(ref_cell),
            _ => None,
        })
    }

    /// Iterate over only the combined line symbols.
    pub fn iter_combined_line_symbols(
        &self,
    ) -> impl Iterator<Item = &Rc<RefCell<CombinedLineSymbol>>> {
        self.symbols.iter().filter_map(|s| match s {
            Symbol::CombinedLine(ref_cell) => Some(ref_cell),
            _ => None,
        })
    }

    /// Iterate over only the combined area symbols.
    pub fn iter_combined_area_symbols(
        &self,
    ) -> impl Iterator<Item = &Rc<RefCell<CombinedAreaSymbol>>> {
        self.symbols.iter().filter_map(|s| match s {
            Symbol::CombinedArea(ref_cell) => Some(ref_cell),
            _ => None,
        })
    }

    /// Returns `true` if the symbol set contains no symbols.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

impl SymbolSet {
    pub(crate) fn symbol_by_index(&self, id: usize) -> Option<&Symbol> {
        if id >= self.len() {
            None
        } else {
            Some(&self.symbols[id])
        }
    }

    pub(crate) fn get_weak_symbol_by_index(&self, id: usize) -> Option<WeakSymbol> {
        self.symbol_by_index(id).map(|c| c.downgrade())
    }

    pub(crate) fn try_sort(&mut self) -> Result<()> {
        let mut codes = Vec::with_capacity(self.len());
        for s in &self.symbols {
            codes.push(s.common()?.code);
        }

        let mut v = self.symbols.iter().cloned().enumerate().collect::<Vec<_>>();
        v.sort_by_key(|(i, _)| codes[*i]);
        self.symbols = v.into_iter().map(|(_, s)| s).collect();

        Ok(())
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
        if symbols.iter().any(|s| s.is_none()) {
            return Err(Error::SymbolCountMismatch);
        }
        let mut symbol_set = Self {
            symbols: symbols
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .ok_or(Error::SymbolCountMismatch)?,
            name: symbol_set_name,
        };

        // Before linking public components, identify CombinedArea symbols
        // that should actually be CombinedLine symbols.
        // At this point, only private parts are populated in the CombinedAreaSymbols.

        // Step 1: Initial candidates — CombinedArea symbols with no private Area parts
        // and whose public component IDs don't reference Area/Point/Text symbols.
        let mut candidate_indices: Vec<usize> = Vec::new();
        for (i, symbol) in symbol_set.symbols.iter().enumerate() {
            if let Symbol::CombinedArea(rc) = symbol {
                let ca = rc.try_borrow()?;
                let has_private_area = ca.components().any(|p| {
                    matches!(p, PublicOrPrivateSymbol::Private(AreaOrLineSymbol::Area(_)))
                });
                if has_private_area {
                    continue;
                }
                let has_area_public = components[i].iter().any(|&id| {
                    matches!(
                        symbol_set.symbols.get(id),
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
                    matches!(symbol_set.symbols.get(id), Some(Symbol::CombinedArea(_)))
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
            let new_symbol = {
                let old_symbol = &symbol_set.symbols[idx];
                if let Symbol::CombinedArea(rc) = old_symbol {
                    let mut ca = rc.try_borrow_mut()?;
                    let common = ca.common.clone();
                    let mut cl = CombinedLineSymbol::new(Code::default(), String::new());
                    cl.common = common;
                    let part_count = ca.components().count();
                    for _ in 0..part_count {
                        if let Some(PublicOrPrivateSymbol::Private(AreaOrLineSymbol::Line(line))) =
                            ca.remove_component(0)
                        {
                            cl.add_component(PublicOrPrivateSymbol::Private(line))?;
                        }
                    }
                    Symbol::CombinedLine(Rc::new(RefCell::new(cl)))
                } else {
                    unreachable!("Candidate index should point to CombinedArea");
                }
            };
            symbol_set.symbols[idx] = new_symbol;
        }

        // Step 4: Link public components for all combined symbols.
        // This runs after conversion so weak references point to the correct types.
        for (component_ids, symbol) in components.iter().zip(&symbol_set.symbols) {
            if component_ids.is_empty() {
                continue;
            }
            match symbol {
                Symbol::CombinedArea(ref_cell) => {
                    let mut symb = ref_cell.try_borrow_mut()?;
                    for &id in component_ids {
                        let weak_component = symbol_set
                            .get_weak_symbol_by_index(id)
                            .ok_or(Error::SymbolSetIndexOutOfRange(id))?;
                        match weak_component {
                            WeakSymbol::Line(weak) => symb.add_component(
                                PublicOrPrivateSymbol::Public(WeakPathSymbol::Line(weak)),
                            )?,
                            WeakSymbol::Area(weak) => symb.add_component(
                                PublicOrPrivateSymbol::Public(WeakPathSymbol::Area(weak)),
                            )?,
                            WeakSymbol::CombinedArea(weak) => symb.add_component(
                                PublicOrPrivateSymbol::Public(WeakPathSymbol::CombinedArea(weak)),
                            )?,
                            WeakSymbol::CombinedLine(weak) => symb.add_component(
                                PublicOrPrivateSymbol::Public(WeakPathSymbol::CombinedLine(weak)),
                            )?,
                            _ => return Err(Error::CombinedSymbolContainsPointOrText),
                        }
                    }
                }
                Symbol::CombinedLine(ref_cell) => {
                    let mut symb = ref_cell.try_borrow_mut()?;
                    for &id in component_ids {
                        let weak_component = symbol_set
                            .get_weak_symbol_by_index(id)
                            .ok_or(Error::SymbolSetIndexOutOfRange(id))?;
                        match weak_component {
                            WeakSymbol::Line(weak) => symb.add_component(
                                PublicOrPrivateSymbol::Public(WeakLinePathSymbol::Line(weak)),
                            )?,
                            WeakSymbol::CombinedLine(weak) => {
                                symb.add_component(PublicOrPrivateSymbol::Public(
                                    WeakLinePathSymbol::CombinedLine(weak),
                                ))?;
                            }
                            _ => return Err(Error::CombinedLineSymbolContainsNonLine),
                        }
                    }
                }
                _ if !component_ids.is_empty() => {
                    return Err(Error::ComponentsInNonCombinedSymbol);
                }
                _ => {}
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
        for (index, symbol) in self.iter().enumerate() {
            symbol.write(writer, self, colors, index)?;
            writer.get_mut().write_all(b"\n".as_slice())?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbols")))?;
        Ok(())
    }
}
