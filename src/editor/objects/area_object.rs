use geo_types::Polygon;

use super::PatternRotation;
use crate::editor::{Error, Result, Transform};

#[derive(Debug, Clone)]
pub struct AreaObject {
    pub polygon: Polygon,
    pub pattern_rotation: PatternRotation,
}

impl AreaObject {
    pub(super) fn write<W: std::io::Write>(
        &self,
        write: &mut W,
        transform: &Transform,
    ) -> std::result::Result<(), std::io::Error> {
    }
}

impl AreaObject {
    fn parse_polygon(coords_str: &str) -> Result<Polygon<f64>> {
        let coords = super::parse_coordinates(coords_str)?;

        if coords.len() < 6 || coords.len() % 2 != 0 {
            return Err(Error::InvalidCoordinate(
                "Polygon needs at least 3 points (6 coordinates)".to_string(),
            ));
        }

        let points: Vec<(f64, f64)> = coords.chunks(2).map(|chunk| (chunk[0], chunk[1])).collect();

        // Close the polygon if it's not already closed
        let mut polygon_points = points;
        if polygon_points.first() != polygon_points.last() {
            if let Some(first) = polygon_points.first() {
                polygon_points.push(*first);
            }
        }

        Ok(Polygon::new(LineString::from(polygon_points), vec![]))
    }
}
