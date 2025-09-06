use std::{io::BufRead, str::FromStr};

use super::{SymbolCode, SymbolType};
use crate::editor::{Error, Result, notes};

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    symbol_type: SymbolType,
    xml_def: String,
    code: SymbolCode,
    pub description: String,
    name: String,
}

impl Symbol {
    pub(super) fn new(
        symbol_type: SymbolType,
        xml_def: String,
        code: SymbolCode,
        description: String,
        name: String,
    ) -> Symbol {
        Symbol {
            symbol_type,
            xml_def,
            code,
            description,
            name,
        }
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
    ) -> Result<Symbol> {
        let mut symbol_type = None;
        let mut description = String::new();
        let mut name = String::new();
        let mut code = None;
        let mut xml_def = String::new();

        // Parse attributes
        for attr in element.attributes() {
            let attr = attr?;

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
                    let mut parts = [0, 0, 0];
                    for (i, part) in std::str::from_utf8(&attr.value)?
                        .split('.')
                        .enumerate()
                        .take(3)
                    {
                        parts[i] = u16::from_str(part)?;
                    }

                    code = Some(SymbolCode::from(parts));
                }
                _ => {}
            }
        }

        if symbol_type.is_none() || code.is_none() || name.is_empty() {
            return Err(Error::ParseOmapFileError(
                "Could not parse symbol".to_string(),
            ));
        }

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

        Ok(Symbol {
            symbol_type: symbol_type.unwrap(),
            code: code.unwrap(),
            description,
            name,
            xml_def,
        })
    }

    pub(super) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(self.xml_def.as_bytes())?;
        Ok(())
    }
}
