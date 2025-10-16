use std::{cell::RefCell, hash::Hash, io::BufRead, rc::Weak, str::FromStr, usize};

use super::{SymbolCode, SymbolType};
use crate::editor::{
    Error, Result,
    colors::{Color, ColorSet},
    notes,
};

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Symbol {
    id: usize,
    symbol_type: SymbolType,
    xml_def: String,
    code: SymbolCode,
    pub description: String,
    name: String,
    colors: Vec<Weak<RefCell<Color>>>,
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.xml_def == other.xml_def
    }
}

impl Eq for Symbol {}

impl Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.xml_def.hash(state);
    }
}

impl Symbol {
    pub(super) fn new(
        symbol_type: SymbolType,
        xml_def: String,
        code: SymbolCode,
        description: String,
        name: String,
        colors: Vec<Weak<RefCell<Color>>>,
        id: usize,
    ) -> Symbol {
        Symbol {
            symbol_type,
            xml_def,
            code,
            description,
            name,
            colors,
            id,
        }
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn get_symbol_type(&self) -> SymbolType {
        self.symbol_type
    }

    pub fn get_code(&self) -> SymbolCode {
        self.code
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl Symbol {
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart,
        color_set: &ColorSet,
    ) -> Result<Symbol> {
        let mut id = usize::MAX;
        let mut symbol_type = None;
        let mut description = String::new();
        let mut name = String::new();
        let mut code = None;
        let mut xml_def = String::new();

        // Parse attributes
        for attr in element.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"type" => {
                    symbol_type = match attr.value.as_ref() {
                        b"1" => Some(SymbolType::Point),
                        b"2" => Some(SymbolType::Line),
                        b"4" => Some(SymbolType::Area),
                        b"8" => Some(SymbolType::Text),
                        _ => None,
                    };
                }
                b"name" => {
                    name = String::from_utf8(attr.value.to_vec())?;
                }
                b"code" => {
                    let parts = std::str::from_utf8(&attr.value)?
                        .split('.')
                        .take(3)
                        .map(|s| u16::from_str(s).unwrap_or(0));

                    code = Some(SymbolCode::from(parts));
                }
                b"id" => {
                    id = usize::from_str(std::str::from_utf8(&attr.value)?)?;
                }
                _ => {}
            }
        }

        if symbol_type.is_none() || code.is_none() || name.is_empty() || id == usize::MAX {
            return Err(Error::ParseOmapFileError(
                "Could not parse symbol".to_string(),
            ));
        }
        let symbol_type = symbol_type.unwrap();
        let code = code.unwrap();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"description" => description = notes::parse(reader)?,
                    name => {
                        xml_def.push_str(
                            format!(
                                "<{}{}>",
                                std::str::from_utf8(name)?,
                                std::str::from_utf8(bytes_start.attributes_raw())?,
                            )
                            .as_str(),
                        );
                        let _ = reader.stream().read_line(&mut xml_def);
                        break;
                    }
                },
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"symbol") {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in symbol parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }

        let mut colors = Vec::new();

        let re = Regex::new("color=\"([-?][0-9+])\"").unwrap();
        for (_, [color_index]) in re.captures_iter(&xml_def).map(|n| n.extract()) {
            let index = i16::from_str(color_index)?;
            if index >= 0 {
                colors.push(color_set.get_weak_color_by_id(index as usize).ok_or(
                    Error::ParseOmapFileError("Bad color in symbol parsing".to_string()),
                )?);
            }
        }

        Ok(Symbol {
            symbol_type,
            code,
            description,
            name,
            xml_def,
            colors,
            id,
        })
    }

    pub(super) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(self.xml_def.as_bytes())?;
        Ok(())
    }
}
