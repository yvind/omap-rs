use super::{AreaObject, LineObject, MapObjectTrait, PointObject, TagTrait, TextObject};
use crate::{symbols::Symbol, transform::Transform, OmapError, OmapResult};
use std::{fs::File, io::BufWriter};

/// Enum for the different map object types
#[derive(Debug, Clone)]
pub enum MapObject {
    /// line object
    LineObject(LineObject),
    /// point object
    PointObject(PointObject),
    /// area object
    AreaObject(AreaObject),
    /// text object
    TextObject(TextObject),
}

impl From<LineObject> for MapObject {
    fn from(value: LineObject) -> Self {
        MapObject::LineObject(value)
    }
}

impl From<AreaObject> for MapObject {
    fn from(value: AreaObject) -> Self {
        MapObject::AreaObject(value)
    }
}

impl From<PointObject> for MapObject {
    fn from(value: PointObject) -> Self {
        MapObject::PointObject(value)
    }
}

impl From<TextObject> for MapObject {
    fn from(value: TextObject) -> Self {
        MapObject::TextObject(value)
    }
}

impl MapObject {
    pub(crate) fn write_to_map(
        self,
        f: &mut BufWriter<File>,
        bezier_error: Option<f64>,
        transform: &Transform,
    ) -> OmapResult<()> {
        match self {
            MapObject::LineObject(line_object) => {
                line_object.write_to_map(f, bezier_error, transform)
            }
            MapObject::PointObject(point_object) => {
                point_object.write_to_map(f, bezier_error, transform)
            }
            MapObject::AreaObject(area_object) => {
                area_object.write_to_map(f, bezier_error, transform)
            }
            MapObject::TextObject(text_object) => {
                text_object.write_to_map(f, bezier_error, transform)
            }
        }
    }

    /// get symbol of a map object
    pub fn symbol(&self) -> Symbol {
        match self {
            MapObject::LineObject(line_object) => line_object.symbol.into(),
            MapObject::PointObject(point_object) => point_object.symbol.into(),
            MapObject::AreaObject(area_object) => area_object.symbol.into(),
            MapObject::TextObject(text_object) => text_object.symbol.into(),
        }
    }

    /// change the symbol of a map object
    /// returns Err MismatchingSymbolAndObject if the object type and symbol type do not match
    /// nothing happens in that case
    pub fn change_symbol(&mut self, new_symbol: impl Into<Symbol>) -> OmapResult<()> {
        let new_symbol = new_symbol.into();

        match (self, new_symbol) {
            (MapObject::LineObject(line_object), Symbol::Line(line_symbol)) => {
                line_object.change_symbol(line_symbol)
            }
            (MapObject::PointObject(point_object), Symbol::Point(point_symbol)) => {
                point_object.change_symbol(point_symbol)
            }
            (MapObject::AreaObject(area_object), Symbol::Area(area_symbol)) => {
                area_object.change_symbol(area_symbol)
            }
            (MapObject::TextObject(text_object), Symbol::Text(text_symbol)) => {
                text_object.change_symbol(text_symbol)
            }
            _ => return Err(OmapError::MismatchingSymbolAndObject),
        };

        Ok(())
    }
}

impl TagTrait for MapObject {
    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>) {
        match self {
            MapObject::LineObject(line_object) => line_object.add_tag(k, v),
            MapObject::PointObject(point_object) => point_object.add_tag(k, v),
            MapObject::AreaObject(area_object) => area_object.add_tag(k, v),
            MapObject::TextObject(text_object) => text_object.add_tag(k, v),
        }
    }
}
