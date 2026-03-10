use std::collections::HashMap;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{AreaObject, LineObject, PointObject, TextObject, write_tags};
use crate::{
    Error, Result,
    objects::{HorizontalAlign, VerticalAlign},
    symbols::{AreaObjectSymbol, LineObjectSymbol, SymbolSet, WeakSymbol},
    utils::parse_attr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ObjectType {
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

    fn tags(&self) -> &HashMap<String, String> {
        match self {
            MapObject::Point(o) => &o.tags,
            MapObject::Line(o) => &o.tags,
            MapObject::Area(o) => &o.tags,
            MapObject::Text(o) => &o.tags,
        }
    }

    pub(crate) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbols: &SymbolSet,
    ) -> Result<()> {
        let mut bs = BytesStart::new("object");
        bs.push_attribute(("type", self.type_value().to_string().as_str()));

        // Write symbol index
        if let Some(idx) = symbols.get_index_of_weak_symbol(&self.get_weak_symbol()) {
            bs.push_attribute(("symbol", idx.to_string().as_str()));
        }

        // Write rotation and text-specific attributes
        match self {
            MapObject::Point(p) => {
                if p.rotation.abs() > f64::EPSILON {
                    bs.push_attribute(("rotation", p.rotation.to_string().as_str()));
                }
            }
            MapObject::Text(t) => {
                if t.rotation.abs() > f64::EPSILON {
                    bs.push_attribute(("rotation", t.rotation.to_string().as_str()));
                }
                bs.push_attribute(("h_align", (t.h_align as u8).to_string().as_str()));
                bs.push_attribute(("v_align", (t.v_align as u8).to_string().as_str()));
            }
            _ => {}
        }

        writer.write_event(Event::Start(bs))?;

        // Write tags if present
        let tags = self.tags();
        if !tags.is_empty() {
            write_tags(writer, tags)?;
        }

        // Write type-specific content (coords + extras)
        match self {
            MapObject::Point(p) => p.write_content(writer)?,
            MapObject::Line(l) => l.write_content(writer)?,
            MapObject::Area(a) => a.write_content(writer)?,
            MapObject::Text(t) => t.write_content(writer)?,
        }

        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
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

        let mut tags = None;

        // Read child elements until we find <coords>, then delegate
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref inner) => match inner.local_name().as_ref() {
                    b"tags" => tags = Some(super::parse_tags(reader)?),
                    b"coords" => {
                        // Delegate to the appropriate object parser.
                        // Each parser reads from inside <coords> through </object>.
                        let mut map_object = match object_type {
                            ObjectType::Point => {
                                MapObject::Point(PointObject::parse(reader, inner, rotation)?)
                            }
                            ObjectType::Line => MapObject::Line(LineObject::parse(reader, inner)?),
                            ObjectType::Area => MapObject::Area(AreaObject::parse(reader, inner)?),
                            ObjectType::Text => MapObject::Text(TextObject::parse(
                                reader, inner, h_align, v_align, rotation,
                            )?),
                        };

                        // Set tags if we parsed any
                        if let Some(t) = tags {
                            match &mut map_object {
                                MapObject::Point(o) => o.tags = t,
                                MapObject::Line(o) => o.tags = t,
                                MapObject::Area(o) => o.tags = t,
                                MapObject::Text(o) => o.tags = t,
                            }
                        }

                        // Set the symbol
                        match (&mut map_object, &weak_symbol) {
                            (MapObject::Point(o), WeakSymbol::Point(w)) => {
                                o.symbol = w.clone();
                            }
                            (MapObject::Line(o), WeakSymbol::Line(w)) => {
                                o.symbol = LineObjectSymbol::Line(w.clone());
                            }
                            (MapObject::Line(o), WeakSymbol::CombinedLine(w)) => {
                                o.symbol = LineObjectSymbol::CombinedLine(w.clone());
                            }
                            (MapObject::Area(o), WeakSymbol::Area(w)) => {
                                o.symbol = AreaObjectSymbol::Area(w.clone());
                            }
                            (MapObject::Area(o), WeakSymbol::CombinedArea(w)) => {
                                o.symbol = AreaObjectSymbol::CombinedArea(w.clone());
                            }
                            (MapObject::Text(o), WeakSymbol::Text(w)) => {
                                o.symbol = w.clone();
                            }
                            _ => {
                                return Err(Error::ParseOmapFileError(
                                    "Symbol type mismatch for object".to_string(),
                                ));
                            }
                        }

                        return Ok(map_object);
                    }
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        return Err(Error::ParseOmapFileError(
                            "Object ended without coords".to_string(),
                        ));
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
    }
}

#[cfg(test)]
mod tests {
    use geo_types::coord;

    #[test]
    fn reverse_line_string_xml() {
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

        let result = super::super::line_object::reverse_raw_line_coords(&in_xml);
        assert_eq!(result, true_out);
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

        flip_xml = super::super::line_object::reverse_raw_line_coords(&flip_xml);
        flip_xml = super::super::line_object::reverse_raw_line_coords(&flip_xml);
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

        flip_xml = super::super::line_object::reverse_raw_line_coords(&flip_xml);
        flip_xml = super::super::line_object::reverse_raw_line_coords(&flip_xml);
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

        flip_xml = super::super::area_object::reverse_raw_polygon_coords(&flip_xml);
        flip_xml = super::super::area_object::reverse_raw_polygon_coords(&flip_xml);
        assert_eq!(in_xml, flip_xml);
    }
}
