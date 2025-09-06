use std::str::FromStr;

use super::{Symbol, SymbolCode, SymbolType};
use crate::editor::{Error, Result, colors::ColorSet};

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

#[derive(Debug, Clone)]
pub struct SymbolSet {
    symbols: Vec<Symbol>,
    id: String,
}

impl SymbolSet {
    /// Get the symbol set name/id
    pub fn get_symbol_set_id(&self) -> &str {
        &self.id
    }

    pub fn get_symbol_by_id(&self, id: usize) -> Option<&Symbol> {
        if self.num_symbols() <= id {
            None
        } else {
            Some(&self.symbols[id])
        }
    }

    pub fn get_symbol_by_code(&self, code: SymbolCode) -> Option<&Symbol> {
        self.symbols.iter().find(|&s| s.get_code() == code)
    }

    pub fn get_symbol_by_name(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|&s| s.get_name() == name)
    }

    /// Access the symbols through an iterator
    pub fn iter(&self) -> std::slice::Iter<'_, Symbol> {
        self.symbols.iter()
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
        color_priority: usize,
        width: u32,
        description: String,
    ) {
        let def = format!(
            "<line_symbol color=\"{color_priority}\" line_width=\"{width}\" join_style=\"2\" cap_style=\"1\"/>",
        );

        self.symbols.push(Symbol::new(
            SymbolType::Line,
            def,
            symbol_code.into(),
            description,
            name,
        ));
    }

    pub fn push_simple_area_symbol(
        &mut self,
        symbol_code: impl Into<SymbolCode>,
        name: String,
        color_priority: usize,
        description: String,
    ) {
        let def = format!("<area_symbol inner_color=\"{color_priority}\"/>");

        self.symbols.push(Symbol::new(
            SymbolType::Area,
            def,
            symbol_code.into(),
            description,
            name,
        ));
    }

    pub fn push_simple_point_symbol(
        &mut self,
        symbol_code: impl Into<SymbolCode>,
        name: String,
        color_priority: usize,
        radius: u32,
        description: String,
    ) {
        let def =
            format!("<point_symbol inner_radius=\"{radius}\" inner_color=\"{color_priority}\"/>",);

        self.symbols.push(Symbol::new(
            SymbolType::Point,
            def,
            symbol_code.into(),
            description,
            name,
        ));
    }

    pub fn push_simple_text_symbol(
        &mut self,
        symbol_code: impl Into<SymbolCode>,
        name: String,
        size: u32,
        color_priority: usize,
        description: String,
    ) {
        let def = format!(
            "<text_symbol icon_text=\"A\"><font family=\"Sans Serif\" size=\"{size}\"/><text color=\"{color_priority}\"/></text_symbol>"
        );

        self.symbols.push(Symbol::new(
            SymbolType::Text,
            def,
            symbol_code.into(),
            description,
            name,
        ));
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

        for attr in element.attributes() {
            let attr = attr?;

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
                        symbols.push(Symbol::parse(reader, &bytes_start)?);
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

        Ok(SymbolSet { symbols, id })
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(
            format!(
                "<symbols count=\"{}\" id=\"{}\">\n",
                self.num_symbols(),
                self.id
            )
            .as_bytes(),
        )?;
        for symbol in self.symbols {
            symbol.write(writer)?;
        }
        writer.write_all("</symbols>\n".as_bytes())?;
        Ok(())
    }
}
