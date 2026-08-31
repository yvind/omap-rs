use std::collections::HashMap;

use quick_xml::{Reader, Writer, events::BytesStart};

use super::{AreaObject, LineObject, PointObject, TextObject};
use crate::{
    Error, Result,
    objects::{HorizontalAlign, VerticalAlign},
    symbols::{Symbol, SymbolId, SymbolKind, SymbolSet},
    utils::parse_attr_raw,
};

/// Narrow the handle an object names to the kind that object can render.
fn narrow<T>(
    symbol: Option<SymbolId>,
    symbols: &SymbolSet,
    expected: &'static [SymbolKind],
    narrow: impl FnOnce(SymbolId) -> Option<T>,
) -> Result<Option<T>> {
    let Some(id) = symbol else {
        return Ok(None);
    };
    match narrow(id) {
        Some(narrowed) => Ok(Some(narrowed)),
        None => Err(Error::SymbolKindMismatch {
            expected,
            found: symbols.get(id).ok_or(Error::SymbolConversionError)?.kind(),
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(super) enum ObjectType {
    Point,
    Line,
    Area,
    Text,
}

/// A map object that can be a point, line, area, or text.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// The symbol this object is rendered with, or `None` for the format's
    /// unknown-symbol sentinel.
    pub fn symbol(&self) -> Option<SymbolId> {
        match self {
            Self::Point(object) => object.symbol.map(Into::into),
            Self::Line(object) => object.symbol.map(Into::into),
            Self::Area(object) => object.symbol.map(Into::into),
            Self::Text(object) => object.symbol.map(Into::into),
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

    /// Transform this object, preserving Bézier control points for line and
    /// area objects.
    pub fn transform<F>(&mut self, transform: F)
    where
        F: Fn(geo_types::Coord) -> geo_types::Coord,
    {
        match self {
            Self::Point(object) => object.transform(transform),
            Self::Line(object) => object.transform(transform),
            Self::Area(object) => object.transform(transform),
            Self::Text(object) => object.transform(transform),
        }
    }

    /// Try to transform this object, preserving Bézier control points for line
    /// and area objects.
    ///
    /// # Errors
    ///
    /// Returns any error produced while transforming the object. The object is
    /// unchanged on failure.
    pub fn try_transform<E, F>(&mut self, transform: F) -> std::result::Result<(), E>
    where
        F: Fn(geo_types::Coord) -> std::result::Result<geo_types::Coord, E>,
    {
        match self {
            Self::Point(object) => object.try_transform(transform),
            Self::Line(object) => object.try_transform(transform),
            Self::Area(object) => object.try_transform(transform),
            Self::Text(object) => object.try_transform(transform),
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

        // An inline element symbol, or the format's -1, both mean `None`.
        let symbol = if let Some(sid) = symbol_id
            && sid >= 0
        {
            Some(
                symbols
                    .id_at(usize::try_from(sid)?)
                    .ok_or(Error::UnknownObjectSymbolId(sid))?,
            )
        } else {
            None
        };

        // Mapper does not distinguish area from line objects; the symbol does.
        if object_type == ObjectType::Area
            && symbol
                .and_then(|id| symbols.get(id))
                .is_some_and(|symbol| matches!(symbol, Symbol::Line(_) | Symbol::CombinedLine(_)))
        {
            object_type = ObjectType::Line;
        }

        match object_type {
            ObjectType::Point => Ok(Self::Point(PointObject::parse(
                reader,
                narrow(symbol, symbols, &[SymbolKind::Point], |id| {
                    symbols.point_id(id)
                })?,
                rotation,
            )?)),
            ObjectType::Line => Ok(Self::Line(LineObject::parse(
                reader,
                narrow(
                    symbol,
                    symbols,
                    &[SymbolKind::Line, SymbolKind::CombinedLine],
                    |id| symbols.line_path_id(id),
                )?,
            )?)),
            // do not bother sending rotation to the AreaObject as it is also given in the pattern rotation
            ObjectType::Area => Ok(Self::Area(AreaObject::parse(
                reader,
                narrow(
                    symbol,
                    symbols,
                    &[SymbolKind::Area, SymbolKind::CombinedArea],
                    |id| symbols.area_path_id(id),
                )?,
            )?)),
            ObjectType::Text => Ok(Self::Text(TextObject::parse(
                reader,
                narrow(symbol, symbols, &[SymbolKind::Text], |id| {
                    symbols.text_id(id)
                })?,
                h_align,
                v_align,
                rotation,
            )?)),
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
    };

    #[test]
    fn rotatable_objects_follow_transform_rotation() -> Result<()> {
        let transform = |coord: geo_types::Coord| coord! { x: -coord.y + 10., y: coord.x - 5. };

        let mut point = PointObject::new(None, Point::new(1., 2.));
        point.rotation = 0.1;
        let mut point = MapObject::Point(point);
        point.transform(transform);
        let MapObject::Point(point) = point else {
            return Err(crate::Error::ObjectError);
        };
        assert!((point.rotation - (0.1 + FRAC_PI_2)).abs() < 1e-12);

        let mut text = TextObject::new(
            None,
            TextGeometry::SingleAnchor(coord! { x: 1., y: 2. }),
            String::new(),
        );
        text.rotation = -0.2;
        let mut text = MapObject::Text(text);
        text.transform(transform);
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
        let mut area = AreaObject::new(None, Polygon::new(ring, Vec::new()));
        area.pattern_rotation.coord = coord! { x: 1., y: 2. };
        area.pattern_rotation.rotation = 0.3;
        let mut area = MapObject::Area(area);
        area.transform(transform);
        let MapObject::Area(area) = area else {
            return Err(crate::Error::ObjectError);
        };
        assert!((area.pattern_rotation.rotation - (0.3 + FRAC_PI_2)).abs() < 1e-12);

        Ok(())
    }
}
