use geo_types::LineString;

use crate::editor::{Error, Result, Transform};

use super::PatternRotation;

#[derive(Debug, Clone)]
pub struct LineObject {
    pub line: LineString,
    pub pattern_rotation: PatternRotation,
}

impl LineObject {
    pub(super) fn write<W: std::io::Write>(
        &self,
        write: &mut W,
        transform: &Transform,
    ) -> std::result::Result<(), std::io::Error> {
    }
}

impl LineObject {
    fn parse_linestring(coords_str: &str) -> Result<LineString<f64>> {
        let coords = super::parse_coordinates(coords_str)?;

        if coords.len() < 4 || coords.len() % 2 != 0 {
            return Err(Error::InvalidCoordinate(
                "LineString needs at least 2 points (4 coordinates)".to_string(),
            ));
        }

        let points: Vec<(f64, f64)> = coords.chunks(2).map(|chunk| (chunk[0], chunk[1])).collect();

        Ok(LineString::from(points))
    }
}
