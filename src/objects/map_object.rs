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
            Self::Point(point_object) => WeakSymbol::Point(point_object.symbol.clone()),
            Self::Line(line_object) => match &line_object.symbol {
                WeakLinePathSymbol::Line(weak) => WeakSymbol::Line(weak.clone()),
                WeakLinePathSymbol::CombinedLine(weak) => WeakSymbol::CombinedLine(weak.clone()),
            },
            Self::Area(area_object) => match &area_object.symbol {
                WeakAreaPathSymbol::Area(weak) => WeakSymbol::Area(weak.clone()),
                WeakAreaPathSymbol::CombinedArea(weak) => WeakSymbol::CombinedArea(weak.clone()),
            },
            Self::Text(text_object) => WeakSymbol::Text(text_object.symbol.clone()),
        }
    }

    /// Get the tags of the object
    pub fn tags(&self) -> &HashMap<String, String> {
        match self {
            Self::Point(o) => &o.tags,
            Self::Line(o) => &o.tags,
            Self::Area(o) => &o.tags,
            Self::Text(o) => &o.tags,
        }
    }

    /// Get mutable tags of the object
    pub fn tags_mut(&mut self) -> &mut HashMap<String, String> {
        match self {
            Self::Point(o) => &mut o.tags,
            Self::Line(o) => &mut o.tags,
            Self::Area(o) => &mut o.tags,
            Self::Text(o) => &mut o.tags,
        }
    }

    /// Get a weak pointer to the objects symbol
    pub fn get_symbol(&self) -> WeakSymbol {
        match self {
            Self::Point(point_object) => WeakSymbol::Point(point_object.symbol.clone()),
            Self::Line(line_object) => match &line_object.symbol {
                WeakLinePathSymbol::Line(weak) => WeakSymbol::Line(weak.clone()),
                WeakLinePathSymbol::CombinedLine(weak) => WeakSymbol::CombinedLine(weak.clone()),
            },
            Self::Area(area_object) => match &area_object.symbol {
                WeakAreaPathSymbol::Area(weak) => WeakSymbol::Area(weak.clone()),
                WeakAreaPathSymbol::CombinedArea(weak) => WeakSymbol::CombinedArea(weak.clone()),
            },
            Self::Text(text_object) => WeakSymbol::Text(text_object.symbol.clone()),
        }
    }

    pub(crate) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        if self.geometry_is_empty() {
            return Ok(());
        }

        match self {
            Self::Point(point_object) => point_object.write(writer, symbol_set)?,
            Self::Line(line_object) => line_object.write(writer, symbol_set)?,
            Self::Area(area_object) => area_object.write(writer, symbol_set)?,
            Self::Text(text_object) => text_object.write(writer, symbol_set)?,
        }
        Ok(())
    }

    pub(crate) fn geometry_is_empty(&self) -> bool {
        match self {
            Self::Line(object) => object.geometry_is_empty(),
            Self::Area(object) => object.geometry_is_empty(),
            Self::Point(_) | Self::Text(_) => false,
        }
    }

    /// Apply a coordinate transform to this object, preserving
    /// Bézier control points for line and area objects.
    ///
    /// # Errors
    ///
    /// Returns any error produced while transforming the object.
    pub fn apply_transform<F>(&mut self, transform: &F) -> Result<()>
    where
        F: Fn(geo_types::Coord) -> Result<geo_types::Coord> + ?Sized,
    {
        match self {
            Self::Point(o) => o.apply_transform(transform),
            Self::Line(o) => o.apply_transform(transform),
            Self::Area(o) => o.apply_transform(transform),
            Self::Text(o) => o.apply_transform(transform),
        }
    }
}

impl From<AreaObject> for MapObject {
    fn from(value: AreaObject) -> Self {
        Self::Area(value)
    }
}

impl From<LineObject> for MapObject {
    fn from(value: LineObject) -> Self {
        Self::Line(value)
    }
}

impl From<PointObject> for MapObject {
    fn from(value: PointObject) -> Self {
        Self::Point(value)
    }
}

impl From<TextObject> for MapObject {
    fn from(value: TextObject) -> Self {
        Self::Text(value)
    }
}

impl MapObject {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        bytes_start: &BytesStart<'_>,
        symbols: &SymbolSet,
        is_line_element: bool,
    ) -> Result<Self> {
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
                b"symbol" => symbol_id = parse_attr_raw::<i32>(attr.value).ok(),
                b"rotation" => rotation = parse_attr_raw(attr.value).unwrap_or(rotation),
                b"h_align" => h_align = parse_attr_raw(attr.value).unwrap_or(h_align),
                b"v_align" => v_align = parse_attr_raw(attr.value).unwrap_or(v_align),
                _ => (),
            }
        }

        let Some(mut object_type) = object_type else {
            return Err(Error::MissingObjectType);
        };

        if is_line_element {
            object_type = ObjectType::Line;
        }

        // for elements the symbol_id is not given as the symbol is given in the element and we need to create a dummy weaksymbol
        // Objects can have symbol id of -1 meaning unknown symbol so create a dummy in that case also
        let weak_symbol = if let Some(sid) = symbol_id
            && sid >= 0
        {
            symbols
                .get_weak_symbol_by_id(sid as usize)
                .ok_or(Error::UnknownObjectSymbolId(sid))?
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
                Ok(Self::Point(PointObject::parse(reader, ps, rotation)?))
            }
            (ObjectType::Line, WeakSymbol::Line(ls)) => Ok(Self::Line(LineObject::parse(
                reader,
                WeakLinePathSymbol::Line(ls),
            )?)),
            (ObjectType::Line, WeakSymbol::CombinedLine(cls)) => Ok(Self::Line(LineObject::parse(
                reader,
                WeakLinePathSymbol::CombinedLine(cls),
            )?)),
            // do not bother sending rotation to the AreaObject as it is also given in the pattern rotation
            (ObjectType::Area, WeakSymbol::Area(ars)) => Ok(Self::Area(AreaObject::parse(
                reader,
                WeakAreaPathSymbol::Area(ars),
            )?)),
            (ObjectType::Area, WeakSymbol::CombinedArea(cas)) => Ok(Self::Area(AreaObject::parse(
                reader,
                WeakAreaPathSymbol::CombinedArea(cas),
            )?)),
            (ObjectType::Text, WeakSymbol::Text(ts)) => Ok(Self::Text(TextObject::parse(
                reader, ts, h_align, v_align, rotation,
            )?)),
            _ => Err(Error::ObjectError),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use geo_types::{LineString, Point, Polygon, coord};

    use super::MapObject;
    use crate::{
        Result,
        objects::{AreaObject, PointObject, TextGeometry, TextObject},
        symbols::WeakAreaPathSymbol,
    };

    #[test]
    fn rotatable_objects_follow_transform_rotation() -> Result<()> {
        let transform = |coord: geo_types::Coord| Ok(coord! { x: -coord.y + 10., y: coord.x - 5. });

        let mut point = PointObject::new(std::rc::Weak::new(), Point::new(1., 2.));
        point.rotation = 0.1;
        let mut point = MapObject::Point(point);
        point.apply_transform(&transform)?;
        let MapObject::Point(point) = point else {
            return Err(crate::Error::ObjectError);
        };
        assert!((point.rotation - (0.1 + FRAC_PI_2)).abs() < 1e-12);

        let mut text = TextObject::new(
            std::rc::Weak::new(),
            TextGeometry::SingleAnchor(coord! { x: 1., y: 2. }),
            String::new(),
        );
        text.rotation = -0.2;
        let mut text = MapObject::Text(text);
        text.apply_transform(&transform)?;
        let MapObject::Text(text) = text else {
            return Err(crate::Error::ObjectError);
        };
        assert!((text.rotation - (-0.2 + FRAC_PI_2)).abs() < 1e-12);

        let ring = LineString::from(vec![
            coord! { x: 0., y: 0. },
            coord! { x: 1., y: 0. },
            coord! { x: 0., y: 1. },
            coord! { x: 0., y: 0. },
        ]);
        let mut area = AreaObject::new(
            WeakAreaPathSymbol::Area(std::rc::Weak::new()),
            Polygon::new(ring, Vec::new()),
        );
        area.pattern_rotation.coord = coord! { x: 1., y: 2. };
        area.pattern_rotation.rotation = 0.3;
        let mut area = MapObject::Area(area);
        area.apply_transform(&transform)?;
        let MapObject::Area(area) = area else {
            return Err(crate::Error::ObjectError);
        };
        assert!((area.pattern_rotation.rotation - (0.3 + FRAC_PI_2)).abs() < 1e-12);

        Ok(())
    }

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
    fn reverse_preserves_dash_flags_on_open_path_vertices() {
        use crate::objects::{COORD_FLAG_CURVE_START, COORD_FLAG_DASH_POINT};

        let in_xml = [
            (coord! { x: 0, y: 0 }, COORD_FLAG_DASH_POINT),
            (
                coord! { x: 1_000, y: 0 },
                COORD_FLAG_CURVE_START | COORD_FLAG_DASH_POINT,
            ),
            (coord! { x: 1_000, y: 1_000 }, 0),
            (coord! { x: 2_000, y: 1_000 }, 0),
            (coord! { x: 2_000, y: 0 }, COORD_FLAG_DASH_POINT),
            (coord! { x: 3_000, y: 0 }, COORD_FLAG_DASH_POINT),
        ];
        let expected = [
            (coord! { x: 3_000, y: 0 }, COORD_FLAG_DASH_POINT),
            (
                coord! { x: 2_000, y: 0 },
                COORD_FLAG_CURVE_START | COORD_FLAG_DASH_POINT,
            ),
            (coord! { x: 2_000, y: 1_000 }, 0),
            (coord! { x: 1_000, y: 1_000 }, 0),
            (coord! { x: 1_000, y: 0 }, COORD_FLAG_DASH_POINT),
            (coord! { x: 0, y: 0 }, COORD_FLAG_DASH_POINT),
        ];

        let reversed = super::super::line_object::reverse_raw_line_coords(&in_xml);
        assert_eq!(reversed, expected);
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
