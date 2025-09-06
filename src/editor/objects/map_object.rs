use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use std::collections::HashMap;
use std::str::FromStr;

use super::{AreaObject, LineObject, ObjectGeometry, PointObject, TextObject};

use crate::editor::symbols::SymbolType;
use crate::editor::{
    Error, Result,
    objects::text_object::{HorizontalAlign, VerticalAlign},
    symbols::{SymbolId, SymbolSet},
};

#[derive(Debug, Clone)]
pub struct MapObject {
    symbol_id: SymbolId,
    pub tags: HashMap<String, String>,
    geometry: ObjectGeometry,
    // store the initial xml so that the object can be written back without being changed if the coords are not changed
    coords_xml_def: String,
    is_coords_touched: bool,
}

impl MapObject {
    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
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
            self.geometry.write(writer)?;
        } else {
            writer.write_all(self.coords_xml_def.as_bytes())?;
        }

        writer.write_all("</object>\n".as_bytes())?;
        Ok(())
    }
}

impl MapObject {
    pub fn get_symbol_id(&self) -> SymbolId {
        self.symbol_id
    }
}

impl MapObject {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        bytes_start: &BytesStart,
        symbols: &SymbolSet,
    ) -> Result<MapObject> {
        let mut object_type = None;
        let mut symbol_id = SymbolId::MAX;
        let mut rotation = 0.;
        let mut h_align = None;
        let mut v_align = None;

        for attr in bytes_start.attributes() {
            let attr = attr?;

            match attr.key.local_name().as_ref() {
                b"type" => match attr.value.as_ref() {
                    b"0" => object_type = Some(SymbolType::Point),
                    b"1" => object_type = Some(SymbolType::Area),
                    b"4" => object_type = Some(SymbolType::Text),
                    _ => (),
                },

                b"symbol" => symbol_id = usize::from_str(std::str::from_utf8(&attr.value)?)?,

                b"rotation" => rotation = f64::from_str(std::str::from_utf8(&attr.value)?)?,
                b"h_align" => h_align = HorizontalAlign::from_bytes(&attr.value),
                b"v_align" => v_align = VerticalAlign::from_bytes(&attr.value),
                _ => (),
            }
        }

        if symbol_id == SymbolId::MAX || object_type.is_none() {
            return Err(Error::ParseOmapFileError(
                "Could not parse object".to_string(),
            ));
        }

        let mut object_type = object_type.unwrap();

        // Mapper does not discern between area and line objects. But we do (Polygon vs LineString in object geometry)!
        if let SymbolType::Area = object_type {
            if let Some(symbol) = symbols.get_symbol_by_id(symbol_id) {
                object_type = symbol.get_symbol_type();
            } else {
                return Err(Error::ParseOmapFileError(
                    "Unknown symbol detected for object".to_string(),
                ));
            }
        }

        let mut geometry = None;
        let mut coords_xml_def = None;
        let mut tags = None;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"tags" => tags = Some(super::parse_tags(reader)?),
                    b"coords" => {
                        (geometry, coords_xml_def) = match object_type {
                            SymbolType::Point => {
                                let (po, xml) = PointObject::parse(reader, rotation)?;
                                (Some(ObjectGeometry::Point(po)), Some(xml))
                            }
                            SymbolType::Line => {
                                let (lo, xml) = LineObject::parse(reader)?;
                                (Some(ObjectGeometry::Line(lo)), Some(xml))
                            }
                            SymbolType::Area | SymbolType::Combined => {
                                let (ao, xml) = AreaObject::parse(reader)?;
                                (Some(ObjectGeometry::Area(ao)), Some(xml))
                            }
                            SymbolType::Text => {
                                let (to, xml) =
                                    TextObject::parse(reader, h_align, v_align, rotation)?;
                                (Some(ObjectGeometry::Text(to)), Some(xml))
                            }
                        };
                        break;
                    }
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in object parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }

        if geometry.is_none() || coords_xml_def.is_none() {
            return Err(Error::ParseOmapFileError(
                "Invalid object geometry".to_string(),
            ));
        }

        Ok(MapObject {
            symbol_id,
            tags: tags.unwrap_or_default(),
            geometry: geometry.unwrap(),
            coords_xml_def: coords_xml_def.unwrap(),
            is_coords_touched: false,
        })
    }
}
