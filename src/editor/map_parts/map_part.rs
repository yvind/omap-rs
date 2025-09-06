use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::editor::objects::MapObject;
use crate::editor::symbols::{SymbolId, SymbolSet};
use crate::editor::{Error, Result};

#[derive(Debug, Clone)]
pub struct MapPart {
    pub name: String,
    pub objects: HashMap<SymbolId, Vec<MapObject>>,
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

        for attr in element.attributes() {
            let attr = attr?;

            if matches!(attr.key.local_name().as_ref(), b"name") {
                name = String::from_utf8(attr.value.to_vec())?;
            }
        }

        let mut objects: HashMap<usize, Vec<MapObject>> = HashMap::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"object") {
                        let object = MapObject::parse(reader, &bytes_start, symbols)?;

                        let symbol_id = object.get_symbol_id();
                        if let Some(contained) = objects.get_mut(&symbol_id) {
                            contained.push(object);
                        } else {
                            let _ = objects.insert(symbol_id, vec![object]);
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
