use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
    str::FromStr,
};

use super::{CombinedSymbolType, Symbol, SymbolCode, SymbolType};
use crate::editor::{
    Error, Result,
    colors::{Color, ColorSet},
};

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SymbolSet {
    symbols: Vec<Rc<RefCell<Symbol>>>,
    name: String,
}

impl SymbolSet {
    /// Get the symbol set name
    pub fn get_symbol_set_name(&self) -> &str {
        &self.name
    }

    pub fn get_symbol_by_id(&self, id: usize) -> Option<Ref<'_, Symbol>> {
        self.symbols
            .iter()
            .filter_map(|s| s.try_borrow().ok())
            .find(|s| s.get_id() == id)
    }

    pub(crate) fn get_weak_symbol_by_id(&self, id: usize) -> Option<Weak<RefCell<Symbol>>> {
        self.symbols
            .iter()
            .find(|&s| s.clone().borrow().get_id() == id)
            .map(Rc::downgrade)
    }

    pub fn get_symbol_by_code(&self, code: SymbolCode) -> Option<Ref<'_, Symbol>> {
        self.symbols
            .iter()
            .filter_map(|s| s.try_borrow().ok())
            .find(|s| s.get_code() == code)
    }

    pub fn get_symbol_by_name(&self, name: &str) -> Option<Ref<'_, Symbol>> {
        self.symbols
            .iter()
            .filter_map(|s| s.try_borrow().ok())
            .find(|s| s.get_name() == name)
    }

    /// Access the symbols through an iterator
    pub fn iter(&self) -> impl Iterator<Item = Result<Ref<'_, Symbol>>> {
        self.symbols.iter().map(|s| {
            let s = s.try_borrow()?;
            Ok(s)
        })
    }

    /// Access the mutable symbols through an iterator
    pub fn iter_mut(&mut self) -> impl Iterator<Item = Result<RefMut<'_, Symbol>>> {
        self.symbols.iter().map(|s| {
            let s = s.try_borrow_mut()?;
            Ok(s)
        })
    }

    /// Get the number of symbol in the symbol set
    pub fn num_symbols(&self) -> usize {
        self.symbols.len()
    }

    /// Add a simple line symbol to the symbol set
    pub fn push_simple_line_symbol(
        &mut self,
        symbol_code: impl Into<SymbolCode>,
        name: String,
        color: Weak<RefCell<Color>>,
        width: u32,
        description: String,
    ) -> Result<()> {
        let id = color
            .upgrade()
            .ok_or(Error::ParseOmapFileError(
                "Could not upgrade color pointer as the data has been dropped".to_string(),
            ))?
            .try_borrow()?
            .get_id();
        let def = format!(
            "<line_symbol color=\"{id}\" line_width=\"{width}\" join_style=\"2\" cap_style=\"1\"/>",
        );

        self.symbols.push(Rc::new(RefCell::new(Symbol::new(
            SymbolType::Line,
            def,
            symbol_code.into(),
            description,
            name,
            vec![color],
            self.num_symbols(),
        ))));
        Ok(())
    }

    pub fn push_simple_area_symbol(
        &mut self,
        symbol_code: impl Into<SymbolCode>,
        name: String,
        color: Weak<RefCell<Color>>,
        description: String,
    ) -> Result<()> {
        let id = color
            .upgrade()
            .ok_or(Error::ParseOmapFileError(
                "Could not upgrade color pointer as the data has been dropped".to_string(),
            ))?
            .try_borrow()?
            .get_id();
        let def = format!("<area_symbol inner_color=\"{id}\"/>");

        self.symbols.push(Rc::new(RefCell::new(Symbol::new(
            SymbolType::Area,
            def,
            symbol_code.into(),
            description,
            name,
            vec![color],
            self.num_symbols(),
        ))));

        Ok(())
    }

    pub fn push_simple_point_symbol(
        &mut self,
        symbol_code: impl Into<SymbolCode>,
        name: String,
        color: Weak<RefCell<Color>>,
        radius: u32,
        description: String,
    ) -> Result<()> {
        let id = color
            .upgrade()
            .ok_or(Error::ParseOmapFileError(
                "Could not upgrade color pointer as the data has been dropped".to_string(),
            ))?
            .try_borrow()?
            .get_id();
        let def = format!("<point_symbol inner_radius=\"{radius}\" inner_color=\"{id}\"/>",);

        self.symbols.push(Rc::new(RefCell::new(Symbol::new(
            SymbolType::Point,
            def,
            symbol_code.into(),
            description,
            name,
            vec![color],
            self.num_symbols(),
        ))));
        Ok(())
    }

    pub fn push_simple_text_symbol(
        &mut self,
        symbol_code: impl Into<SymbolCode>,
        name: String,
        size: u32,
        color: Weak<RefCell<Color>>,
        description: String,
    ) -> Result<()> {
        let id = color
            .upgrade()
            .ok_or(Error::ParseOmapFileError(
                "Could not upgrade color pointer as the data has been dropped".to_string(),
            ))?
            .try_borrow()?
            .get_id();

        let def = format!(
            "<text_symbol icon_text=\"A\"><font family=\"Sans Serif\" size=\"{size}\"/><text color=\"{id}\"/></text_symbol>"
        );

        self.symbols.push(Rc::new(RefCell::new(Symbol::new(
            SymbolType::Text,
            def,
            symbol_code.into(),
            description,
            name,
            vec![color],
            self.num_symbols(),
        ))));
        Ok(())
    }
}

impl SymbolSet {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart,
        colors: &ColorSet,
    ) -> Result<SymbolSet> {
        let mut id = String::new();
        let mut count = 0;

        for attr in element.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"id" => id = attr.unescape_value()?.into_owned(),
                b"count" => count = usize::from_str(&attr.unescape_value()?)?,
                _ => (),
            }
        }

        let mut symbols = Vec::with_capacity(count);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"symbol") {
                        symbols.push(Rc::new(RefCell::new(Symbol::parse(
                            reader,
                            &bytes_start,
                            colors,
                        )?)));
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

        let symbol_set = SymbolSet { symbols, name: id };

        let part_symbol_re = Regex::new("<part symbol=\"([0-9]+)\"/>").unwrap();
        let private_symbol_re =
            Regex::new("<part private\"true\"><symbol type=\"([0-9])\"").unwrap();

        'symbols: for symbol in symbol_set.symbols.iter() {
            let mut s = symbol.try_borrow_mut()?;
            if let SymbolType::Combined(_) = s.symbol_type {
                // we must check if the symbol contains only line type sub-symbols and should be used with LineObjects
                // easiest is to check for any area symbols and if so we know it should be used for AreaObjects
                for (_, [private_type]) in private_symbol_re
                    .captures_iter(&s.xml_def)
                    .map(|n| n.extract())
                {
                    if private_type == "4" {
                        s.symbol_type = SymbolType::Combined(CombinedSymbolType::Area);
                        continue 'symbols;
                    }
                }
                for (_, [shared_symbol]) in part_symbol_re
                    .captures_iter(&s.xml_def)
                    .map(|n| n.extract())
                {
                    let symbol_id = usize::from_str(shared_symbol).unwrap();
                    if let Some(symbol) = symbol_set.get_symbol_by_id(symbol_id)
                        && let SymbolType::Area = symbol.get_type()
                    {
                        s.symbol_type = SymbolType::Combined(CombinedSymbolType::Area);
                        continue 'symbols;
                    }
                }
            }
        }

        Ok(symbol_set)
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(
            format!(
                "<symbols count=\"{}\" id=\"{}\">\n",
                self.num_symbols(),
                self.name
            )
            .as_bytes(),
        )?;
        for symbol in self.symbols {
            Rc::into_inner(symbol)
                .ok_or(Error::ParseOmapFileError(
                    "Stray reference to symbol somewhere".to_string(),
                ))?
                .into_inner()
                .write(writer)?;
        }
        writer.write_all("</symbols>\n".as_bytes())?;
        Ok(())
    }
}
