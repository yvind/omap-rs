use super::{SymbolCode, SymbolId, SymbolType};
use crate::editor::Result;

use quick_xml::events::BytesStart;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    symbol_type: SymbolType,
    xml_def: String,
    id: SymbolId,
    code: SymbolCode,
    pub description: String,
    name: String,
}

impl Symbol {
    pub(super) fn new(
        symbol_type: SymbolType,
        xml_def: String,
        id: SymbolId,
        code: SymbolCode,
        description: String,
        name: String,
    ) -> Symbol {
        Symbol {
            symbol_type,
            xml_def,
            id,
            code,
            description,
            name,
        }
    }

    pub fn get_symbol_type(&self) -> SymbolType {
        self.symbol_type
    }

    pub fn get_id(&self) -> SymbolId {
        self.id
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
    pub(super) fn parse_symbol(element: &BytesStart) -> Result<Symbol> {
        todo!();

        /*
        let mut symbol_type = SymbolType::Point;
        let mut definition = String::new();
        let mut description = String::new();
        let mut name = String::new();

        // Parse attributes
        for attr in element.attributes() {
            let attr = attr?;
            let key = std::str::from_utf8(attr.key.as_ref())?;
            let value = std::str::from_utf8(&attr.value)?;

            match key {
                "type" => {
                    symbol_type = match value {
                        "1" => SymbolType::Point,
                        "2" => SymbolType::Line,
                        "4" => SymbolType::Area,
                        "8" => SymbolType::Text,
                        _ => SymbolType::Area,
                    };
                }
                "id" => {
                    definition = value.to_string();
                }
                "name" => {
                    name = value.to_string();
                }
                "code" => {
                    if name.is_empty() {
                        name = format!("Symbol {}", value);
                    }
                }
                _ => {}
            }
        }

        Ok(Symbol {
            symbol_type,
            definition,
            description,
            name,
        })
         */
    }

    pub(super) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(self.xml_def.as_bytes())?;
        Ok(())
    }
}
