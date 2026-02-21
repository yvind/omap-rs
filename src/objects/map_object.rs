use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesStart, Event},
};

use super::{AreaObject, LineObject, PointObject, TextObject};
use crate::{
    Error, Result,
    objects::{HorizontalAlign, VerticalAlign},
    parse_attr,
    symbols::{AreaObjectSymbol, LineObjectSymbol, SymbolSet, WeakSymbol},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectType {
    Point,
    Line,
    Area,
    Text,
}

#[derive(Debug, Clone)]
pub enum MapObject {
    Point(PointObject),
    Line(LineObject),
    Area(AreaObject),
    Text(TextObject),
}

impl MapObject {
    fn type_value(&self) -> u8 {
        match self {
            MapObject::Point(_) => 0,
            MapObject::Area(_) => 1,
            MapObject::Line(_) => 1,
            MapObject::Text(_) => 4,
        }
    }

    pub fn get_weak_symbol(&self) -> WeakSymbol {
        match self {
            MapObject::Point(point_object) => WeakSymbol::Point(point_object.symbol.clone()),
            MapObject::Line(line_object) => match &line_object.symbol {
                LineObjectSymbol::Line(weak) => WeakSymbol::Line(weak.clone()),
                LineObjectSymbol::CombinedLine(weak) => WeakSymbol::CombinedLine(weak.clone()),
            },
            MapObject::Area(area_object) => match &area_object.symbol {
                AreaObjectSymbol::Area(weak) => WeakSymbol::Area(weak.clone()),
                AreaObjectSymbol::CombinedArea(weak) => WeakSymbol::CombinedArea(weak.clone()),
            },
            MapObject::Text(text_object) => WeakSymbol::Text(text_object.symbol.clone()),
        }
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        match self {
            MapObject::Area(area_object) => area_object.write(writer),
            MapObject::Line(line_object) => line_object.write(writer),
            MapObject::Point(point_object) => point_object.write(writer),
            MapObject::Text(text_object) => text_object.write(writer),
        }
    }
}

impl MapObject {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        bytes_start: &BytesStart<'_>,
        symbols: &SymbolSet,
    ) -> Result<MapObject> {
        let mut object_type = None;
        let mut symbol_id = None;
        let mut rotation = 0.;
        let mut h_align = HorizontalAlign::default();
        let mut v_align = VerticalAlign::default();

        for attr in bytes_start.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"type" => match attr.value.as_ref() {
                    b"0" => object_type = Some(ObjectType::Point),
                    b"1" => object_type = Some(ObjectType::Area),
                    b"4" => object_type = Some(ObjectType::Text),
                    _ => (),
                },
                b"symbol" => symbol_id = parse_attr(attr.value),
                b"rotation" => rotation = parse_attr(attr.value).unwrap_or(rotation),
                b"h_align" => h_align = parse_attr(attr.value).unwrap_or(h_align),
                b"v_align" => v_align = parse_attr(attr.value).unwrap_or(v_align),
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

        let weak_symbol =
            symbols
                .get_weak_symbol_by_id(symbol_id)
                .ok_or(Error::ParseOmapFileError(
                    "Unknown Symbol in MapObject parsing".to_string(),
                ))?;

        // Mapper does not discern between area and line objects. But we do because we want a Polygon or a LineString!
        // Let's check the symbol for what the object must be
        if object_type == ObjectType::Area {
            match weak_symbol {
                WeakSymbol::Line(_) | WeakSymbol::CombinedLine(_) => object_type = ObjectType::Line,
                _ => (),
            }
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
                                let (po, xml) = PointObject::parse(reader, rotation)?;
                                coords_xml_string.push_str(&xml);
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
                                let (to, xml) = TextObject::parse(
                                    reader,
                                    h_align,
                                    v_align,
                                    rotation,
                                    &bytes_start,
                                )?;
                                coords_xml_string.push_str(&xml);
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

        let coords = coords_xml_string
            .split_terminator(';')
            .map(|s| -> Option<MapCoord> {
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

        let raw_geometry = coords.into_iter().flatten().collect::<Vec<_>>();

        Ok(MapObject {
            symbol: weak_symbol,
            tags: tags.unwrap_or_default(),
            geometry,
            is_coords_touched: false,
            raw_map_coords: raw_geometry,
        })
    }
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
