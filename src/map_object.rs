use std::{fs::File, io::BufWriter};

use crate::{AreaObject, LineObject, OmapResult, PointObject, Scale};

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

pub trait TagTrait {
    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>);

    fn add_auto_tag(&mut self) {
        self.add_tag("auto-generated", "OmapMaker");
    }
}

pub enum MapObject {
    LineObject(LineObject),
    PointObject(PointObject),
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
}

impl TagTrait for MapObject {
    fn add_auto_tag(&mut self) {
        match self {
            MapObject::LineObject(line_object) => line_object.add_auto_tag(),
            MapObject::PointObject(point_object) => point_object.add_auto_tag(),
            MapObject::AreaObject(area_object) => area_object.add_auto_tag(),
        }
    }

    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>) {
        match self {
            MapObject::LineObject(line_object) => line_object.add_tag(k, v),
            MapObject::PointObject(point_object) => point_object.add_tag(k, v),
            MapObject::AreaObject(area_object) => area_object.add_tag(k, v),
        }
    }
}
