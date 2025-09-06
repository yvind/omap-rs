use geo_types::Point;
use quick_xml::Reader;

use crate::editor::{Error, Result, Transform};

#[derive(Debug, Clone)]
pub struct PointObject {
    pub point: Point,
    pub rotation: f64,
}

impl PointObject {
    pub(super) fn get_special_keys(&self) -> Option<String> {
        Some(format!("rotation=\"{}\"", self.rotation))
    }

    pub(super) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        let map_coords = transform.to_map_coords(self.point.0);
        writer.write_all(
            format!(
                "<coords count=\"1\">{} {};</coords>",
                map_coords.0, map_coords.1
            )
            .as_bytes(),
        )?;
        Ok(())
    }
}

impl PointObject {
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        rotation: f64,
    ) -> Result<(Self, String)> {
        let coords: Vec<f64> = super::parse_coordinates(coords_str)?;

        if coords.len() >= 2 {
            Ok(Point::new(coords[0], coords[1]))
        } else {
            Err(Error::InvalidCoordinate(
                "Point needs at least 2 coordinates".to_string(),
            ))
        }
    }
}
