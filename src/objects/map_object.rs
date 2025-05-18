use super::{AreaObject, LineObject, MapObjectTrait, PointObject, TagTrait, TextObject};
use crate::{symbols::Symbol, OmapResult, Scale};
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
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<()> {
        match self {
            MapObject::LineObject(line_object) => line_object.write_to_map(
                f,
                bezier_error,
                scale,
                grivation,
                inv_combined_scale_factor,
            ),
            MapObject::PointObject(point_object) => point_object.write_to_map(
                f,
                bezier_error,
                scale,
                grivation,
                inv_combined_scale_factor,
            ),
            MapObject::AreaObject(area_object) => area_object.write_to_map(
                f,
                bezier_error,
                scale,
                grivation,
                inv_combined_scale_factor,
            ),
            MapObject::TextObject(text_object) => text_object.write_to_map(
                f,
                bezier_error,
                scale,
                grivation,
                inv_combined_scale_factor,
            ),
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
