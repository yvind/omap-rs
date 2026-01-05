use geo_types::Coord;
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use std::{cell::RefCell, collections::HashMap, rc::Weak, str::FromStr};

use super::{AreaObject, LineObject, ObjectGeometry, PointObject, TextObject};

use crate::editor::symbols::{CombinedSymbolType, Symbol, SymbolType};
use crate::editor::{
    Error, Result,
    objects::text_object::{HorizontalAlign, VerticalAlign},
    symbols::SymbolSet,
};

#[derive(Debug, Clone)]
pub struct MapObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    // these two fields should probably be linked in some way so that the symbol type always matches the geometry type
    pub symbol: Weak<RefCell<Symbol>>,
    geometry: ObjectGeometry,
    // store the raw map-file coords with flags so that the object can be written back unchanged if the coords are untouched
    // (so that the errors introduced when mapping from beziers to linestring and back only are introduced when necessary)
    raw_map_coords: Vec<(Coord<i32>, u8)>,
    is_coords_touched: bool,
}

impl MapObject {
    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        if let Some(symbol) = self.symbol.upgrade() {
            writer.write_all(
                format!(
                    "<object type=\"{}\" symbol=\"{}\"",
                    self.geometry.type_value(),
                    symbol.try_borrow()?.get_id()
                )
                .as_bytes(),
            )?;
        } else {
            return Err(Error::ParseOmapFileError(
                "Invalid symbol pointer".to_string(),
            ));
        }

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
        }
        todo!();
        /*
         else {
            writer.write_all(self.coords_xml_def.as_bytes())?;
        }

        writer.write_all("</object>\n".as_bytes())?;
        Ok(())
        */
    }
}

impl MapObject {
    /// Borrow the geometry
    pub fn get_geometry(&self) -> &ObjectGeometry {
        &self.geometry
    }

    /// Borrow the geometry as mutable, marks the geometry as edited
    pub fn get_geometry_mut(&mut self) -> &mut ObjectGeometry {
        self.is_coords_touched = true;

        &mut self.geometry
    }

    /// Borrow the raw geometry in map-file coords, including flags
    /// (map-file coords are in integer micrometers of paper relative the ref point
    /// with positive x east and positive y south)
    pub fn get_raw_geometry(&self) -> &[(Coord<i32>, u8)] {
        &self.raw_map_coords
    }

    /// Borrow the raw geometry in map-file coords, including flags, as mutable
    ///
    /// This is often not what you want to do
    ///
    /// (map-file coords are in integer micrometers of paper relative the ref point
    /// with positive x east and positive y south)
    pub fn get_raw_geometry_mut(&mut self) -> &mut [(Coord<i32>, u8)] {
        &mut self.raw_map_coords
    }

    /// Reverses a geometry and the input xml without marking it as touched
    pub fn reverse_geometry(&mut self) {
        match (&mut self.geometry, &mut self.raw_map_coords) {
            (ObjectGeometry::Area(area_object), xml) => {
                area_object.polygon.exterior_mut(|e| e.0.reverse());
                area_object
                    .polygon
                    .interiors_mut(|is| is.iter_mut().for_each(|i| i.0.reverse()));

                *xml = reverse_raw_polygon_coords(xml);
            }
            (ObjectGeometry::Line(line_object), xml) => {
                line_object.line.0.reverse();
                *xml = reverse_raw_line_coords(xml);
            }
            _ => (),
        }
    }

    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        bytes_start: &BytesStart<'_>,
        symbols: &SymbolSet,
    ) -> Result<MapObject> {
        let mut object_type = None;
        let mut symbol_id = None;
        let mut rotation = 0.;
        let mut h_align = None;
        let mut v_align = None;

        for attr in bytes_start.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"type" => match attr.value.as_ref() {
                    b"0" => object_type = Some(SymbolType::Point),
                    b"1" => object_type = Some(SymbolType::Area),
                    b"4" => object_type = Some(SymbolType::Text),
                    _ => (),
                },

                b"symbol" => symbol_id = Some(usize::from_str(std::str::from_utf8(&attr.value)?)?),

                b"rotation" => rotation = f64::from_str(std::str::from_utf8(&attr.value)?)?,
                b"h_align" => h_align = HorizontalAlign::from_bytes(&attr.value),
                b"v_align" => v_align = VerticalAlign::from_bytes(&attr.value),
                _ => (),
            }
        }

        if symbol_id.is_none() || object_type.is_none() {
            return Err(Error::ParseOmapFileError(
                "Could not parse object".to_string(),
            ));
        }
        let mut object_type = object_type.unwrap();
        let symbol_id = symbol_id.unwrap();

        let symbol = symbols
            .get_weak_symbol_by_id(symbol_id)
            .ok_or(Error::ParseOmapFileError(
                "Unknown Symbol in MapObject parsing".to_string(),
            ))?;

        // Mapper does not discern between area and line objects. But we do (Polygon vs LineString in object geometry)!
        if let SymbolType::Area = object_type {
            object_type = symbol
                .upgrade()
                .ok_or(Error::ParseOmapFileError(
                    "Unknown Symbol in MapObject parsing".to_string(),
                ))?
                .try_borrow()?
                .get_symbol_type();
        }

        let mut geometry = None;
        let mut coords_xml_string = String::new();
        let mut tags = None;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"tags" => tags = Some(super::parse_tags(reader)?),
                    b"coords" => {
                        geometry = match object_type {
                            SymbolType::Point => {
                                let po = PointObject::parse(reader, rotation)?;
                                Some(ObjectGeometry::Point(po))
                            }
                            SymbolType::Line | SymbolType::Combined(CombinedSymbolType::Line) => {
                                let (lo, xml) = LineObject::parse(reader, &bytes_start)?;
                                coords_xml_string.push_str(&xml);
                                Some(ObjectGeometry::Line(lo))
                            }
                            SymbolType::Area | SymbolType::Combined(CombinedSymbolType::Area) => {
                                let (ao, xml) = AreaObject::parse(reader, &bytes_start)?;
                                coords_xml_string.push_str(&xml);
                                Some(ObjectGeometry::Area(ao))
                            }
                            SymbolType::Text => {
                                let to = TextObject::parse(
                                    reader,
                                    h_align,
                                    v_align,
                                    rotation,
                                    &bytes_start,
                                )?;
                                Some(ObjectGeometry::Text(to))
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

        let geometry = match geometry {
            Some(g) => g,
            None => {
                return Err(Error::ParseOmapFileError(
                    "Invalid object geometry".to_string(),
                ));
            }
        };

        let coords = coords_xml_string[0..coords_xml_string.len() - 1] // drop the last ;
            .split(';')
            .map(|s| -> Option<(Coord<i32>, u8)> {
                let parts = s.split_whitespace().collect::<Vec<_>>();
                match parts.len() {
                    2 => {
                        let x = parts[0].parse();
                        let y = parts[1].parse();
                        if let Ok(x) = x
                            && let Ok(y) = y
                        {
                            Some((Coord { x, y }, 0_u8))
                        } else {
                            None
                        }
                    }
                    3 => {
                        let x = parts[0].parse();
                        let y = parts[1].parse();
                        let f = parts[2].parse();

                        if let Ok(x) = x
                            && let Ok(y) = y
                            && let Ok(f) = f
                        {
                            Some((Coord { x, y }, f))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        if coords.iter().any(|option| option.is_none()) {
            return Err(Error::ParseOmapFileError(
                "Could not parse coords and flags from xml string".to_string(),
            ));
        }

        let coords = coords.into_iter().flatten().collect::<Vec<_>>();

        Ok(MapObject {
            symbol,
            tags: tags.unwrap_or_default(),
            geometry,
            is_coords_touched: coords.is_empty(),
            raw_map_coords: coords,
        })
    }
}

fn reverse_raw_line_coords(coords: &[(Coord<i32>, u8)]) -> Vec<(Coord<i32>, u8)> {
    // iterate through and check the flags
    // flags 1, 2 and 16 must be moved
    // 2 and 16 can only exist at the end and must be moved there
    // flag 1 must be moved to the other end of the bezier
    let mut new_xml = Vec::with_capacity(coords.len());

    let mut end_flag = 0;
    for i in (0..coords.len()).rev() {
        let (coord, mut flag) = coords[i];
        // remove a possible bezier flag
        flag -= flag & 1;

        if i == coords.len() - 1 {
            end_flag += (flag & 2) + (flag & 16);
            flag -= end_flag;
        }
        if i > 2 {
            // check the flag of i + 3 for a 1
            let (_, bez_flag) = coords[i - 3];
            flag |= bez_flag & 1;
        } else if i == 0 {
            flag |= end_flag;
        }
        new_xml.push((coord, flag));
    }
    new_xml
}

fn reverse_raw_polygon_coords(coords: &[(Coord<i32>, u8)]) -> Vec<(Coord<i32>, u8)> {
    // get each of the substrings for each loop and flip them
    // a substring ends with a 2 flag (often 18 or 50)
    let mut s = Vec::with_capacity(coords.len());
    let mut prev_split = 0;
    for (i, (_, f)) in coords.iter().enumerate() {
        if f & 2 == 2 || i == coords.len() - 1 {
            s.extend(reverse_raw_line_coords(&coords[prev_split..=i]));
            prev_split = i + 1;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use geo_types::coord;

    #[test]
    fn reverse_line_string_xml() {
        let mut in_xml = [
            (coord! {x: -11535, y: -1901}, 1),
            (coord! {x:-12228, y: -1077}, 0),
            (coord! {x:-12122,y: 154}, 0),
            (coord! {x:-11297, y: 847}, 1),
            (coord! {x:-10473, y: 1541}, 0),
            (coord! {x:-9242, y: 1435}, 0),
            (coord! {x:-8549, y: 610}, 4),
            (coord! {x: -7855, y: -215}, 0),
            (coord! {x: -7961, y: -1445}, 0),
            (coord! {x:-8786, y: -2139}, 1),
            (coord! {x: -9611, y: -2832}, 0),
            (coord! {x:-10841, y: -2726}, 0),
            (coord! {x:-11535 , y:-1901}, 18),
        ]
        .to_vec();
        let true_out = [
            (coord! {x: -11535, y: -1901}, 1),
            (coord! {x:-10841, y: -2726}, 0),
            (coord! {x:-9611,y: -2832}, 0),
            (coord! {x:-8786, y: -2139}, 0),
            (coord! {x:-7961, y: -1445}, 0),
            (coord! {x:-7855, y: -215}, 0),
            (coord! {x:-8549, y: 610}, 5),
            (coord! {x: -9242, y: 1435}, 0),
            (coord! {x: -10473, y: 1541}, 0),
            (coord! {x:-11297, y: 847}, 1),
            (coord! {x: -12122, y: 154}, 0),
            (coord! {x:-12228, y: -1077}, 0),
            (coord! {x:-11535 , y:-1901}, 18),
        ]
        .to_vec();

        in_xml = super::reverse_raw_line_coords(&in_xml);
        assert_eq!(in_xml, true_out);
    }

    #[test]
    fn reverse_weird_flags() {
        let in_xml = [
            (coord! {x: 11691, y: -14574}, 32),
            (coord! {x: 43270, y: -14766}, 32),
            (coord! {x: 43429, y: 11462}, 0),
            (coord! {x: 11850, y: 11654}, 32),
            (coord! {x: 11691, y: -14574}, 50),
        ]
        .to_vec();
        let mut flip_xml = in_xml.clone();

        flip_xml = super::reverse_raw_line_coords(&flip_xml);
        flip_xml = super::reverse_raw_line_coords(&flip_xml);
        assert_eq!(in_xml, flip_xml);
    }

    #[test]
    fn reverse_line_string_xml_twice() {
        let in_xml = [
            (coord! {x: -11535, y: -1901}, 1),
            (coord! {x:-12228, y: -1077}, 0),
            (coord! {x:-12122,y: 154}, 0),
            (coord! {x:-11297, y: 847}, 1),
            (coord! {x:-10473, y: 1541}, 0),
            (coord! {x:-9242, y: 1435}, 0),
            (coord! {x:-8549, y: 610}, 4),
            (coord! {x: -7855, y: -215}, 0),
            (coord! {x: -7961, y: -1445}, 0),
            (coord! {x:-8786, y: -2139}, 1),
            (coord! {x: -9611, y: -2832}, 0),
            (coord! {x:-10841, y: -2726}, 0),
            (coord! {x:-11535 , y:-1901}, 18),
        ]
        .to_vec();
        let mut flip_xml = in_xml.clone();

        flip_xml = super::reverse_raw_line_coords(&flip_xml);
        flip_xml = super::reverse_raw_line_coords(&flip_xml);
        assert_eq!(in_xml, flip_xml);
    }

    #[test]
    fn reverse_polygon_xml_twice() {
        let in_xml = [
            (coord! { x: -3868, y: 10122}, 1),
            (coord! { x: -10892, y: 7576}, 0),
            (coord! { x: -10555, y: 5582}, 4),
            (coord! { x: -9266, y: 5214}, 4),
            (coord! { x: -7671, y: 3987}, 32),
            (coord! { x: -6291, y: -890}, 0),
            (coord! { x: -4359, y: -1289}, 0),
            (coord! { x: -3868, y: 10122}, 18),
            (coord! { x: -8286, y: 6799}, 0),
            (coord! { x: -5446, y: 7881}, 32),
            (coord! { x: -5968, y: 4055}, 4),
            (coord! { x: -8286, y: 6799}, 18),
        ]
        .to_vec();
        let mut flip_xml = in_xml.clone();

        flip_xml = super::reverse_raw_polygon_coords(&flip_xml);
        flip_xml = super::reverse_raw_polygon_coords(&flip_xml);
        assert_eq!(in_xml, flip_xml);
    }
}
