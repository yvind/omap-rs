use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use std::{collections::HashMap, io::BufRead};

use super::{AreaObject, LineObject, ObjectGeometry, PointObject, TextObject};

use crate::editor::{Error, Result};
use crate::editor::{
    Transform,
    symbols::{Symbol, SymbolType},
};

#[derive(Debug, Clone)]
pub struct MapObject {
    symbol_id: usize,
    pub tags: HashMap<String, String>,
    geometry: ObjectGeometry,
    // store the initial xml so that the object can be written back without being changed if the coords are not changed
    coords_xml_def: String,
    is_coords_touched: bool,
}

impl MapObject {
    pub(crate) fn write<W: std::io::Write>(
        self,
        writer: &mut W,
        transform: &Transform,
    ) -> Result<()> {
        writer.write_all(
            format!(
                "<object type=\"{}\" symbol=\"{}\"",
                self.geometry.type_value(),
                self.symbol_id
            )
            .as_bytes(),
        )?;

        if let Some(str) = self.geometry.get_special_keys() {
            writer.write_all(format!(" {str}>").as_bytes())?;
        } else {
            writer.write_all(">".as_bytes())?;
        }

        if !self.tags.is_empty() {
            writer.write_all("<tags>".as_bytes())?;
            for (key, value) in self.tags {
                writer.write_all(format!("<t k=\"{key}\">{value}</t>").as_bytes())?;
            }
            writer.write_all("</tags>".as_bytes())?;
        }

        if self.is_coords_touched {
            self.geometry.write(writer, transform)?;
        } else {
            writer.write_all(self.coords_xml_def.as_bytes())?;
        }

        writer.write_all("</object>\n".as_bytes())?;
        Ok(())
    }
}

impl MapObject {
    pub fn get_symbol_id(&self) -> usize {
        self.symbol_id
    }
}

impl MapObject {
    pub(crate) fn parse_object<R: BufRead>(
        element: &BytesStart,
        symbol_map: &HashMap<String, Symbol>,
        xml_reader: &mut Reader<R>,
        buf: &mut Vec<u8>,
    ) -> Result<MapObject> {
        let mut tags = HashMap::new();
        let mut symbol_id = String::new();
        let mut object_type_str = String::new();
        let mut rotation: Option<f64> = None;

        // Parse object attributes
        for attr in element.attributes() {
            let attr = attr?;
            let key = std::str::from_utf8(attr.key.as_ref())?;
            let value = std::str::from_utf8(&attr.value)?;

            match key {
                "symbol" => {
                    symbol_id = value.to_string();
                    tags.insert("symbol_id".to_string(), value.to_string());
                }
                "type" => {
                    object_type_str = value.to_string();
                }
                "rotation" => {
                    rotation = value.parse().ok();
                }
                _ => {
                    tags.insert(key.to_string(), value.to_string());
                }
            }
        }

        // Get the symbol from the map
        let symbol = symbol_map
            .get(&symbol_id)
            .cloned()
            .unwrap_or_else(|| Symbol {
                symbol_type: SymbolType::Point,
                definition: symbol_id.clone(),
                description: String::new(),
                name: format!("Unknown symbol {}", symbol_id),
            });

        // Parse object content
        let mut coords_str = String::new();
        let mut text_content = String::new();

        loop {
            match xml_reader.read_event_into(buf)? {
                Event::Start(ref e) => {
                    let tag_name = std::str::from_utf8(e.local_name().into_inner())?;
                    match tag_name {
                        "coords" => {
                            // Read coordinates
                            loop {
                                match xml_reader.read_event_into(buf)? {
                                    Event::Text(e) => {
                                        coords_str.push_str(e.decode()?.as_ref());
                                        break;
                                    }
                                    Event::End(_) => break,
                                    _ => {}
                                }
                            }
                        }
                        "text" => {
                            // Read text content
                            loop {
                                match xml_reader.read_event_into(buf)? {
                                    Event::Text(e) => {
                                        text_content.push_str(e.decode()?.as_ref());
                                        break;
                                    }
                                    Event::End(_) => break,
                                    _ => {}
                                }
                            }
                        }
                        "tags" => {
                            // Parse tags
                            loop {
                                match xml_reader.read_event_into(buf)? {
                                    Event::Start(ref tag_elem) => {
                                        if std::str::from_utf8(tag_elem.local_name().as_ref())?
                                            == "t"
                                        {
                                            let mut key = String::new();
                                            let mut value = String::new();

                                            for attr in tag_elem.attributes() {
                                                let attr = attr?;
                                                let attr_key =
                                                    std::str::from_utf8(attr.key.as_ref())?;
                                                let attr_value = std::str::from_utf8(&attr.value)?;

                                                if attr_key == "k" {
                                                    key = attr_value.to_string();
                                                }
                                            }

                                            // Read tag value
                                            loop {
                                                match xml_reader.read_event_into(buf)? {
                                                    Event::Text(e) => {
                                                        value.push_str(e.decode()?.as_ref());
                                                        break;
                                                    }
                                                    Event::End(_) => break,
                                                    _ => {}
                                                }
                                            }

                                            if !key.is_empty() {
                                                tags.insert(key, value);
                                            }
                                        }
                                    }
                                    Event::End(ref e) => {
                                        if std::str::from_utf8(e.local_name().as_ref())? == "tags" {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {
                            // Skip other elements
                        }
                    }
                }
                Event::End(ref e) => {
                    if std::str::from_utf8(e.local_name().as_ref())? == "object" {
                        break;
                    }
                }
                _ => {}
            }
            buf.clear();
        }

        // Create object based on type
        let object_type = match object_type_str.as_str() {
            "0" => {
                // Point object
                let point = PointObject::parse_point(&coords_str)?;
                ObjectGeometry::Point(PointObject { point })
            }
            "1" => {
                // Line object
                let line = LineObject::parse_linestring(&coords_str)?;
                ObjectGeometry::Line(LineObject { line })
            }
            "2" => {
                // Area object
                let polygon = AreaObject::parse_polygon(&coords_str)?;
                ObjectGeometry::Area(AreaObject { polygon })
            }
            "4" => {
                // Text object
                let point = PointObject::parse_point(&coords_str)?;
                ObjectGeometry::Text(TextObject {
                    point,
                    text: text_content,
                })
            }
            _ => {
                // Default to point based on symbol type
                match symbol.get_symbol_type() {
                    SymbolType::Point => {
                        let point = PointObject::parse_point(&coords_str)?;
                        ObjectGeometry::Point(PointObject { point })
                    }
                    SymbolType::Line => {
                        let line = LineObject::parse_linestring(&coords_str)?;
                        ObjectGeometry::Line(LineObject { line })
                    }
                    SymbolType::Area => {
                        let polygon = AreaObject::parse_polygon(&coords_str)?;
                        ObjectGeometry::Area(AreaObject { polygon })
                    }
                    SymbolType::Text => {
                        let point = PointObject::parse_point(&coords_str)?;
                        ObjectGeometry::Text(TextObject {
                            point,
                            text: text_content,
                        })
                    }
                }
            }
        };

        Ok(MapObject {
            symbol,
            tags,
            object: object_type,
        })
    }
}
