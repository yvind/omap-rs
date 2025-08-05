mod area_object;
mod line_object;
mod point_object;
mod text_object;

mod map_object;

use area_object::AreaObject;
use geo_types::Coord;
use line_object::LineObject;
use point_object::PointObject;
use text_object::TextObject;

pub use map_object::MapObject;

use super::{Error, Result, Transform};

#[derive(Debug, Clone)]
pub enum ObjectGeometry {
    Area(AreaObject),
    Line(LineObject),
    Point(PointObject),
    Text(TextObject),
}

impl ObjectGeometry {
    fn type_value(&self) -> u8 {
        match self {
            ObjectGeometry::Point(_) => 0,
            ObjectGeometry::Area(_) => 1,
            ObjectGeometry::Line(_) => 1,
            ObjectGeometry::Text(_) => 4,
        }
    }

    fn get_special_keys(&self) -> Option<String> {
        match self {
            ObjectGeometry::Point(point_object) => point_object.get_special_keys(),
            ObjectGeometry::Text(text_object) => text_object.get_special_keys(),
            _ => None,
        }
    }

    fn write<W: std::io::Write>(
        self,
        write: &mut W,
        transform: &Transform,
    ) -> std::result::Result<(), std::io::Error> {
        match self {
            ObjectGeometry::Area(area_object) => area_object.write(write, transform),
            ObjectGeometry::Line(line_object) => line_object.write(write, transform),
            ObjectGeometry::Point(point_object) => point_object.write(write, transform),
            ObjectGeometry::Text(text_object) => text_object.write(write, transform),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PatternRotation {
    rotation: f64,
    coord: Coord,
}

fn parse_coordinates(coords_str: &str) -> Result<Vec<f64>> {
    if coords_str.is_empty() {
        return Ok(vec![]);
    }

    let mut coordinates = Vec::new();

    // Split by semicolon first, then by space
    for coord_pair in coords_str.split(';') {
        let parts: Vec<&str> = coord_pair.trim().split_whitespace().collect();

        if parts.len() >= 2 {
            // Parse x and y coordinates
            let x = parts[0].parse::<f64>().map_err(|_| {
                Error::InvalidCoordinate(format!("Invalid x coordinate: {}", parts[0]))
            })?;
            let y = parts[1].parse::<f64>().map_err(|_| {
                Error::InvalidCoordinate(format!("Invalid y coordinate: {}", parts[1]))
            })?;

            coordinates.push(x);
            coordinates.push(y);
        }
    }

    Ok(coordinates)
}
