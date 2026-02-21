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
        CombinedLineSymbol, PubOrPrivSymbol,
        combined_area_symbol::{PathSymbol, WeakPathSymbol},
    },
    try_get_attr,
};

#[derive(Debug, Clone)]
pub struct SymbolSet {
    pub symbols: Vec<Symbol>,
    pub name: String,
}

impl SymbolSet {
    pub fn get_symbol_by_id(&self, id: usize) -> Option<&Symbol> {
        if self.num_symbols() >= id {
            None
        } else {
            Some(&self.symbols[id])
        }
    }

    pub(crate) fn get_weak_symbol_by_id(&self, id: usize) -> Option<WeakSymbol> {
        self.get_symbol_by_id(id).map(|c| c.into())
    }

    pub fn get_symbol_by_code(&self, code: Code) -> Option<&Symbol> {
        self.symbols
            .iter()
            .find(|s| s.get_code().map(|c| c == code).unwrap_or(false))
    }

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

    pub fn iter_weak(&self) -> impl Iterator<Item = WeakSymbol> {
        self.symbols.iter().map(|s| s.into())
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
        let count = try_get_attr(element, "count").unwrap_or(1);

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

        for (component_ids, symbol) in components.into_iter().zip(&symbol_set.symbols) {
            if component_ids.is_empty() {
                continue;
            }
            if let Symbol::CombinedArea(ref_cell) = symbol {
                for id in component_ids {
                    let weak_comp = symbol_set
                        .get_weak_symbol_by_id(id)
                        .ok_or(Error::SymbolError)?;
                    let mut symb = ref_cell.try_borrow_mut()?;
                    match weak_comp {
                        WeakSymbol::Line(weak) => symb
                            .parts
                            .push(PubOrPrivSymbol::Public(WeakPathSymbol::Line(weak))),
                        WeakSymbol::Area(weak) => symb
                            .parts
                            .push(PubOrPrivSymbol::Public(WeakPathSymbol::Area(weak))),
                        _ => return Err(Error::SymbolError),
                    }
                }
            } else {
                return Err(Error::ParseOmapFileError(
                    "Found components in a non-combined symbol".to_string(),
                ));
            }
        }
        // now check if any CombinedArea symbol consist of only lines
        let mut combined_line_ids = Vec::new();
        'symbol: for (id, symbol) in symbol_set.iter().enumerate() {
            if let Symbol::CombinedArea(rc) = symbol {
                let s = rc.try_borrow()?;
                for part in &s.parts {
                    match part {
                        PubOrPrivSymbol::Public(public) => {
                            if let WeakPathSymbol::Area(_) = public {
                                continue 'symbol;
                            }
                        }
                        PubOrPrivSymbol::Private(private) => {
                            if let PathSymbol::Area(_) = private {
                                continue 'symbol;
                            }
                        }
                    }
                }
                // if we reach here we have found a symbol that should be a CombinedLine
                combined_line_ids.push(id);
            }
        }
        for id in combined_line_ids {
            let new_symbol = {
                let symbol = symbol_set.symbols[id].clone();
                if let Symbol::CombinedArea(rc) = symbol {
                    let ca = rc.try_borrow()?;

                    let mut new_parts = Vec::with_capacity(ca.parts.len());
                    for part in &ca.parts {
                        match part {
                            PubOrPrivSymbol::Public(weak_path) => {
                                if let WeakPathSymbol::Line(weak) = weak_path {
                                    new_parts.push(PubOrPrivSymbol::Public(weak.clone()));
                                } else {
                                    return Err(Error::SymbolError);
                                }
                            }
                            PubOrPrivSymbol::Private(path) => {
                                if let PathSymbol::Line(line) = path {
                                    new_parts.push(PubOrPrivSymbol::Private(line.clone()));
                                } else {
                                    return Err(Error::SymbolError);
                                }
                            }
                        }
                    }

                    CombinedLineSymbol {
                        common: ca.common.clone(),
                        parts: new_parts,
                    }
                } else {
                    return Err(Error::SymbolError);
                }
            };
            symbol_set.symbols[id] = Symbol::CombinedLine(Rc::new(RefCell::new(new_symbol)));
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

        for (index, symbol) in self.iter().enumerate() {
            symbol.write(writer, &self, colors, index)?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbols")))?;
        Ok(())
    }
}
