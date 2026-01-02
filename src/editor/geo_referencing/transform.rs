use geo_types::Coord;

use crate::editor::geo_referencing::GeoRef;

const MAP_COORD_MAX: f64 = i32::MAX as f64;

#[derive(Debug, Clone)]
pub struct Transform {
    scale_factor: f64,
    sin: f64,
    cos: f64,
}

impl Transform {
    pub fn to_map_coords(&self, coord: Coord) -> Option<Coord<i32>> {
        let mut x = coord.x * self.cos - coord.y * self.sin;
        let mut y = coord.x * self.sin + coord.y * self.cos;

        x = (x * self.scale_factor).round();
        y = (y * self.scale_factor).round();

        if x.abs() > MAP_COORD_MAX || y.abs() > MAP_COORD_MAX {
            None
        } else {
            Some(Coord {
                x: x as i32,
                y: y as i32,
            })
        }
    }

    pub fn to_proj_coords(&self, coord: Coord<i32>) -> Coord {
        let x_f64 = coord.x as f64 * self.scale_factor;
        let y_f64 = coord.y as f64 * self.scale_factor;

        let x = x_f64 * self.cos - y_f64 * self.sin;
        let y = x_f64 * self.sin + y_f64 * self.cos;

        Coord { x, y }
    }

    pub fn new(geo_ref: &GeoRef) -> Self {
        Self {
            sin: geo_ref.grivation.sin(),
            cos: geo_ref.grivation.cos(),
            scale_factor: geo_ref.combined_scale_factor * geo_ref.scale as f64,
        }
    }
}
