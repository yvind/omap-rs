use geo_types::Coord;

use super::GeoRef;
use crate::{Error, Result};

const FILE_COORD_MAX: f64 = ((i32::MAX / 1000) - 1) as f64;

#[derive(Debug, Clone)]
pub struct Transform {
    map_center: Coord,
    proj_center: Coord,
    scale_factor: f64,
    sin: f64,
    cos: f64,
}

impl Transform {
    pub fn to_map_coords(&self, proj_coord: Coord) -> Coord {
        let (x, mut y) = ((proj_coord - self.proj_center) / self.scale_factor).x_y();

        let x_r = x * self.cos - y * self.sin;
        y = x * self.sin + y * self.cos;

        Coord { x: x_r, y } + self.map_center
    }

    pub fn to_proj_coords(&self, map_coord: Coord) -> Coord {
        let (x, mut y) = ((map_coord - self.map_center) * self.scale_factor).x_y();

        // we want to rotate other way so flip the signs of the sins
        let x_r = x * self.cos + y * self.sin;
        y = -x * self.sin + y * self.cos;

        Coord { x: x_r, y } + self.proj_center
    }

    pub(crate) fn to_file_coords(map_coord: Coord) -> Result<Coord<i32>> {
        if map_coord.x.abs() >= FILE_COORD_MAX || map_coord.y.abs() >= FILE_COORD_MAX {
            return Err(Error::TransfromError);
        } else {
            Ok(Coord {
                x: (map_coord.x * 1000.).round() as i32,
                y: -(map_coord.y * 1000.).round() as i32,
            })
        }
    }

    pub(crate) fn from_file_coords(file_coord: Coord<i32>) -> Coord {
        Coord {
            x: file_coord.x as f64 / 1000.,
            y: -file_coord.y as f64 / 1000.,
        }
    }

    pub(super) fn from_geo_ref(geo_ref: &GeoRef) -> Self {
        Self {
            map_center: geo_ref.map_ref_point,
            proj_center: geo_ref.projected_ref_point,
            sin: geo_ref.grivation_deg.to_radians().sin(),
            cos: geo_ref.grivation_deg.to_radians().cos(),
            scale_factor: geo_ref.combined_scale_factor * geo_ref.scale_denominator as f64 / 1000.,
        }
    }
}
