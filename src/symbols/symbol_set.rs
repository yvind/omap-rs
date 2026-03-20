use std::{cell::RefCell, rc::Rc};

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{Symbol, WeakSymbol};
use crate::{
    Code, Error, Result,
    colors::ColorSet,
    symbols::{
        AreaOrLineSymbol, CombinedLineSymbol, PubOrPrivSymbol, WeakLinePathSymbol, WeakPathSymbol,
    },
    utils::{try_get_attr, try_get_attr_raw},
};

/// An ordered collection of symbols.
#[derive(Debug, Clone)]
pub struct SymbolSet {
    /// The symbols in this set.
    pub symbols: Vec<Symbol>,
    /// The name of the symbol set.
    pub name: String,
}

impl SymbolSet {
    /// Get a symbol by its index in the set.
    pub fn get_symbol_by_id(&self, id: usize) -> Option<&Symbol> {
        if self.num_symbols() <= id {
            None
        } else {
            Some(&self.symbols[id])
        }
    }

    pub(crate) fn get_weak_symbol_by_id(&self, id: usize) -> Option<WeakSymbol> {
        self.get_symbol_by_id(id).map(|c| c.downgrade())
    }

    /// Find a symbol by its code.
    pub fn get_symbol_by_code(&self, code: Code) -> Option<&Symbol> {
        self.symbols
            .iter()
            .find(|s| s.get_code().map(|c| c == code).unwrap_or(false))
    }

    /// Find a symbol by its display name. The first match is returned
    /// If a symbol cannot be borrowed for name checking (because it is mutably borrowed somewhere else)
    /// it is simply skipped. This means that a symbol-name that actually exists in symbol set can return None in some cases
    pub fn get_symbol_by_name(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| match s {
            Symbol::Line(ref_cell) => ref_cell
                .try_borrow()
                .map(|s| s.get_name() == name)
                .unwrap_or(false),
            Symbol::Area(ref_cell) => ref_cell
                .try_borrow()
                .map(|s| s.get_name() == name)
                .unwrap_or(false),
            Symbol::Point(ref_cell) => ref_cell
                .try_borrow()
                .map(|s| s.get_name() == name)
                .unwrap_or(false),
            Symbol::Text(ref_cell) => ref_cell
                .try_borrow()
                .map(|s| s.get_name() == name)
                .unwrap_or(false),
            Symbol::CombinedArea(ref_cell) => ref_cell
                .try_borrow()
                .map(|s| s.get_name() == name)
                .unwrap_or(false),
            Symbol::CombinedLine(ref_cell) => ref_cell
                .try_borrow()
                .map(|s| s.get_name() == name)
                .unwrap_or(false),
        })
    }

    /// Iterate over non-owning references to all symbols.
    pub fn iter_weak(&self) -> impl Iterator<Item = WeakSymbol> {
        self.symbols.iter().map(|s| s.downgrade())
    }

    /// Access the symbols through an iterator
    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    /// Access the mutable symbols through an iterator
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Symbol> {
        self.symbols.iter_mut()
    }

    /// Get the number of symbol in the symbol set
    pub fn num_symbols(&self) -> usize {
        self.symbols.len()
    }
}

impl SymbolSet {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        colors: &ColorSet,
    ) -> Result<SymbolSet> {
        let symbol_set_name = try_get_attr(element, "id").unwrap_or("Custom".to_string());
        let count = try_get_attr_raw(element, "count").unwrap_or(1);

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
                            return Err(Error::ParseOmapFileError(
                                "Found a symbol with an id greater than the number of symbols"
                                    .to_string(),
                            ));
                        }
                        if symbols[symbol_id].is_some() {
                            return Err(Error::ParseOmapFileError(format!(
                                "Found multiple symbols with id={symbol_id}"
                            )));
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
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in symbols parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }
        if symbols.iter().any(|s| s.is_none()) {
            return Err(Error::ParseOmapFileError(
                "The symbol count and number of found symbols do not match".to_string(),
            ));
        }
        let mut symbol_set = SymbolSet {
            symbols: symbols.into_iter().map(|s| s.unwrap()).collect(),
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
                let has_private_area = ca
                    .components()
                    .any(|p| matches!(p, PubOrPrivSymbol::Private(AreaOrLineSymbol::Area(_))));
                if has_private_area {
                    continue;
                }
                let has_area_public = components[i].iter().any(|&id| {
                    matches!(
                        symbol_set.symbols.get(id),
                        Some(Symbol::Area(_)) | Some(Symbol::Point(_)) | Some(Symbol::Text(_))
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
                        if let Some(PubOrPrivSymbol::Private(AreaOrLineSymbol::Line(line))) =
                            ca.remove_component(0)
                        {
                            cl.add_component(PubOrPrivSymbol::Private(line))?;
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
                        let weak_component =
                            symbol_set
                                .get_weak_symbol_by_id(id)
                                .ok_or(Error::SymbolError(format!(
                                    "Symbol set index {id} out of range"
                                )))?;
                        match weak_component {
                            WeakSymbol::Line(weak) => symb.add_component(
                                PubOrPrivSymbol::Public(WeakPathSymbol::Line(weak)),
                            )?,
                            WeakSymbol::Area(weak) => symb.add_component(
                                PubOrPrivSymbol::Public(WeakPathSymbol::Area(weak)),
                            )?,
                            WeakSymbol::CombinedArea(weak) => symb.add_component(
                                PubOrPrivSymbol::Public(WeakPathSymbol::CombinedArea(weak)),
                            )?,
                            WeakSymbol::CombinedLine(weak) => symb.add_component(
                                PubOrPrivSymbol::Public(WeakPathSymbol::CombinedLine(weak)),
                            )?,
                            e => {
                                return Err(Error::SymbolError(format!(
                                    "A combined symbol contains a point or text symbol {:?}",
                                    e
                                )));
                            }
                        }
                    }
                }
                Symbol::CombinedLine(ref_cell) => {
                    let mut symb = ref_cell.try_borrow_mut()?;
                    for &id in component_ids {
                        let weak_component =
                            symbol_set
                                .get_weak_symbol_by_id(id)
                                .ok_or(Error::SymbolError(format!(
                                    "Symbol set index {id} out of range"
                                )))?;
                        match weak_component {
                            WeakSymbol::Line(weak) => symb.add_component(
                                PubOrPrivSymbol::Public(WeakLinePathSymbol::Line(weak)),
                            )?,
                            WeakSymbol::CombinedLine(weak) => symb.add_component(
                                PubOrPrivSymbol::Public(WeakLinePathSymbol::CombinedLine(weak)),
                            )?,
                            e => {
                                return Err(Error::SymbolError(format!(
                                    "A combined line symbol contains a non-line symbol {:?}",
                                    e
                                )));
                            }
                        }
                    }
                }
                _ if !component_ids.is_empty() => {
                    return Err(Error::ParseOmapFileError(
                        "Found components in a non-combined symbol".to_string(),
                    ));
                }
                _ => {}
            }
        }

        Ok(symbol_set)
    }

    pub(crate) fn write<W: std::io::Write>(
        self,
        writer: &mut Writer<W>,
        colors: &ColorSet,
    ) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("symbols").with_attributes([
            ("count", self.num_symbols().to_string().as_str()),
            ("name", self.name.as_str()),
        ])))?;
        writer.get_mut().write_all(b"\n".as_slice())?;
        for (index, symbol) in self.iter().enumerate() {
            symbol.write(writer, &self, colors, index)?;
            writer.get_mut().write_all(b"\n".as_slice())?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbols")))?;
        Ok(())
    }
}
