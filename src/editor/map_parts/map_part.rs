use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::editor::objects::MapObject;
use crate::editor::symbols::{Symbol, SymbolSet};
use crate::editor::{Error, Result};

#[derive(Debug, Clone)]
pub struct MapPart {
    pub name: String,
    pub objects: HashMap<Rc<Symbol>, Vec<MapObject>>, // checkout weaktables crate
}

impl MapPart {
    pub(super) fn merge(&mut self, other: Self) {
        self.objects.extend(other.objects);
    }
}

impl MapPart {
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart,
        symbols: &SymbolSet,
    ) -> Result<MapPart> {
        let mut name = String::new();

        for attr in element.attributes().filter_map(std::result::Result::ok) {
            if matches!(attr.key.local_name().as_ref(), b"name") {
                name = String::from_utf8(attr.value.to_vec())?;
            }
        }

        let mut objects: HashMap<Rc<RefCell<Symbol>>, Vec<MapObject>> = HashMap::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"object") {
                        let object = MapObject::parse(reader, &bytes_start, symbols)?;

                        let symbol =
                            object
                                .get_symbol()
                                .upgrade()
                                .ok_or(Error::ParseOmapFileError(
                                    "Unknown symbol in parsed object".to_string(),
                                ))?;

                        if let Some(contained) = objects.get_mut(symbol.as_ref()) {
                            contained.push(object);
                        } else {
                            let _ = objects.insert(symbol, vec![object]);
                        }
                    }
                }
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"part") {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in parsing MapPart".to_string(),
                    ));
                }
                _ => (),
            }
        }

        Ok(MapPart { name, objects })
    }

    pub(super) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(
            format!(
                "<part name=\"{}\"><objects count=\"{}\">\n",
                self.name,
                self.objects.len()
            )
            .as_bytes(),
        )?;

        for (_, objects) in self.objects {
            for object in objects {
                object.write(writer)?;
            }
        }

        writer.write_all("</objects></part>\n".as_bytes())?;
        Ok(())
    }
}
