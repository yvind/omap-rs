use crate::{AreaObject, LineObject, OmapResult, PointObject, Scale, Symbol};
use std::{fs::File, io::BufWriter};

pub(crate) trait MapObjectTrait {
    fn write_to_map(
        self,
        f: &mut BufWriter<File>,
        bezier_error: Option<f64>,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()>;

    fn write_coords(
        self,
        f: &mut BufWriter<File>,
        bezier_error: Option<f64>,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()>;

    fn write_tags(&self, f: &mut BufWriter<File>) -> OmapResult<()>;
}

/// trait for adding tags to objects
pub trait TagTrait {
    /// add any tag
    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>);

    /// add an elevation tag
    fn add_elevation_tag(&mut self, elevation: f64) {
        self.add_tag("Elevation", format!("{:.2}", elevation));
    }
}

/// Enum for the different map object types
#[derive(Debug, Clone)]
pub enum MapObject {
    /// line object
    LineObject(LineObject),
    /// point object
    PointObject(PointObject),
    /// area object
    AreaObject(AreaObject),
}

impl MapObject {
    pub(crate) fn write_to_map(
        self,
        f: &mut BufWriter<File>,
        bezier_error: Option<f64>,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()> {
        match self {
            MapObject::LineObject(line_object) => {
                line_object.write_to_map(f, bezier_error, scale, grivation, combined_scale_factor)
            }
            MapObject::PointObject(point_object) => {
                point_object.write_to_map(f, bezier_error, scale, grivation, combined_scale_factor)
            }
            MapObject::AreaObject(area_object) => {
                area_object.write_to_map(f, bezier_error, scale, grivation, combined_scale_factor)
            }
        }
    }

    /// get symbol of map object
    pub fn symbol(&self) -> Symbol {
        match self {
            MapObject::LineObject(line_object) => Symbol::from(line_object.symbol),
            MapObject::PointObject(point_object) => Symbol::from(point_object.symbol),
            MapObject::AreaObject(area_object) => Symbol::from(area_object.symbol),
        }
    }
}

impl TagTrait for MapObject {
    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>) {
        match self {
            MapObject::LineObject(line_object) => line_object.add_tag(k, v),
            MapObject::PointObject(point_object) => point_object.add_tag(k, v),
            MapObject::AreaObject(area_object) => area_object.add_tag(k, v),
        }
    }
}
