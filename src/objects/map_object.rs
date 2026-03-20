use std::{collections::HashMap, rc::Weak};

use quick_xml::{Reader, Writer, events::BytesStart};

use super::{AreaObject, LineObject, PointObject, TextObject};
use crate::{
    Error, Result,
    objects::{HorizontalAlign, VerticalAlign},
    symbols::{SymbolSet, WeakAreaPathSymbol, WeakLinePathSymbol, WeakSymbol},
    utils::parse_attr_raw,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ObjectType {
    Point,
    Line,
    Area,
    Text,
}

/// A map object that can be a point, line, area, or text.
#[derive(Debug, Clone)]
pub enum MapObject {
    /// A point object.
    Point(PointObject),
    /// A line object.
    Line(LineObject),
    /// An area object.
    Area(AreaObject),
    /// A text object.
    Text(TextObject),
}

impl MapObject {
    /// Get a non-owning reference to the symbol associated with this object.
    pub fn get_weak_symbol(&self) -> WeakSymbol {
        match self {
            MapObject::Point(point_object) => WeakSymbol::Point(point_object.symbol.clone()),
            MapObject::Line(line_object) => match &line_object.symbol {
                WeakLinePathSymbol::Line(weak) => WeakSymbol::Line(weak.clone()),
                WeakLinePathSymbol::CombinedLine(weak) => WeakSymbol::CombinedLine(weak.clone()),
            },
            MapObject::Area(area_object) => match &area_object.symbol {
                WeakAreaPathSymbol::Area(weak) => WeakSymbol::Area(weak.clone()),
                WeakAreaPathSymbol::CombinedArea(weak) => WeakSymbol::CombinedArea(weak.clone()),
            },
            MapObject::Text(text_object) => WeakSymbol::Text(text_object.symbol.clone()),
        }
    }

    /// Get the tags of the object
    pub fn tags(&self) -> &HashMap<String, String> {
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
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        match self {
            MapObject::Point(point_object) => point_object.write(writer, symbol_set)?,
            MapObject::Line(line_object) => line_object.write(writer, symbol_set)?,
            MapObject::Area(area_object) => area_object.write(writer, symbol_set)?,
            MapObject::Text(text_object) => text_object.write(writer, symbol_set)?,
        }
        Ok(())
    }
}

impl From<AreaObject> for MapObject {
    fn from(value: AreaObject) -> Self {
        MapObject::Area(value)
    }
}

impl From<LineObject> for MapObject {
    fn from(value: LineObject) -> Self {
        MapObject::Line(value)
    }
}

impl From<PointObject> for MapObject {
    fn from(value: PointObject) -> Self {
        MapObject::Point(value)
    }
}

impl From<TextObject> for MapObject {
    fn from(value: TextObject) -> Self {
        MapObject::Text(value)
    }
}

impl MapObject {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        bytes_start: &BytesStart<'_>,
        symbols: &SymbolSet,
        is_line_element: bool,
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
                b"symbol" => symbol_id = parse_attr_raw::<i32>(attr.value),
                b"rotation" => rotation = parse_attr_raw(attr.value).unwrap_or(rotation),
                b"h_align" => h_align = parse_attr_raw(attr.value).unwrap_or(h_align),
                b"v_align" => v_align = parse_attr_raw(attr.value).unwrap_or(v_align),
                _ => (),
            }
        }

        if object_type.is_none() {
            return Err(Error::ParseOmapFileError(
                "Could not parse object type".to_string(),
            ));
        }
        let mut object_type = object_type.unwrap();
        if is_line_element {
            object_type = ObjectType::Line
        }

        // for elements the symbol_id is not given as the symbol is given in the element and we need to create a dummy weaksymbol
        // Objects can have symbol id of -1 meaning unknown symbol so create a dummy in that case also
        let weak_symbol = if let Some(sid) = symbol_id
            && sid >= 0
        {
            symbols
                .get_weak_symbol_by_id(sid as usize)
                .ok_or(Error::ParseOmapFileError(format!(
                    "Unknown Symbol id: {sid} in Object parsing"
                )))?
        } else {
            match object_type {
                ObjectType::Point => WeakSymbol::Point(Weak::new()),
                ObjectType::Line => WeakSymbol::Line(Weak::new()),
                ObjectType::Area => WeakSymbol::Area(Weak::new()),
                ObjectType::Text => WeakSymbol::Text(Weak::new()),
            }
        };

        // Mapper does not discern between area and line objects. But we do because we want a Polygon or a LineString!
        // Let's check the symbol for what the object must be
        if object_type == ObjectType::Area {
            match weak_symbol {
                WeakSymbol::Line(_) | WeakSymbol::CombinedLine(_) => object_type = ObjectType::Line,
                _ => (),
            }
        }

        match (object_type, weak_symbol) {
            (ObjectType::Point, WeakSymbol::Point(ps)) => {
                Ok(MapObject::Point(PointObject::parse(reader, ps, rotation)?))
            }
            (ObjectType::Line, WeakSymbol::Line(ls)) => Ok(MapObject::Line(LineObject::parse(
                reader,
                WeakLinePathSymbol::Line(ls),
            )?)),
            (ObjectType::Line, WeakSymbol::CombinedLine(cls)) => Ok(MapObject::Line(
                LineObject::parse(reader, WeakLinePathSymbol::CombinedLine(cls))?,
            )),
            // do not bother sending rotation to the AreaObject as it is also given in the pattern rotation
            (ObjectType::Area, WeakSymbol::Area(ars)) => Ok(MapObject::Area(AreaObject::parse(
                reader,
                WeakAreaPathSymbol::Area(ars),
            )?)),
            (ObjectType::Area, WeakSymbol::CombinedArea(cas)) => Ok(MapObject::Area(
                AreaObject::parse(reader, WeakAreaPathSymbol::CombinedArea(cas))?,
            )),
            (ObjectType::Text, WeakSymbol::Text(ts)) => Ok(MapObject::Text(TextObject::parse(
                reader, ts, h_align, v_align, rotation,
            )?)),
            _ => Err(Error::ObjectError),
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
