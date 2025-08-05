use geo_types::Coord;

use crate::editor::geo_ref::GeoRef;

pub struct Transform {
    scale: u32,
};

impl Transform {
    pub fn to_map_coords(&self, coord: Coord) -> (i32, i32) {}
    pub fn to_map_dist(&self, dist_in_meters: f64) -> u32 {}
    pub fn new(geo_ref: &GeoRef) -> Self {}
}
