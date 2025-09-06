use geo_types::{LineString, Polygon};
use quick_xml::Reader;

use super::PatternRotation;
use crate::editor::{Error, Result, Transform};

#[derive(Debug, Clone)]
pub struct AreaObject {
    pub polygon: Polygon,
    pub pattern_rotation: PatternRotation,
}

impl AreaObject {
    pub(super) fn write<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        Ok(())
    }
}

impl AreaObject {
    pub(super) fn parse<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<(Self, String)> {
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
