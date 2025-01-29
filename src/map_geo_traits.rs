use geo_types::Coord;

use crate::{OmapError, OmapResult, Scale};

pub(crate) trait MapCoord {
    fn to_map_coordinates(self, scale: Scale, grivation: f32) -> OmapResult<(i32, i32)>;
}

// 1 map unit is 0.001mm on paper => 1000 mu = 1mm on map = 15m on ground
const CONVERSION_15000: f64 = 1_000. / 15.;
const CONVERSION_10000: f64 = 1_000. / 10.;
const CONVERSION_7500: f64 = 1_000. / 7.5;

const MAX_MU: f64 = 2147483647.; // 2^31 - 1 max number a i32 can hold

impl MapCoord for Coord {
    fn to_map_coordinates(self, scale: Scale, _grivation: f32) -> OmapResult<(i32, i32)> {
        let (x, y) = match scale {
            Scale::S7_500 => (
                (self.x * CONVERSION_7500).round(),
                -(self.y * CONVERSION_7500).round(),
            ),
            Scale::S10_000 => (
                (self.x * CONVERSION_10000).round(),
                -(self.y * CONVERSION_10000).round(),
            ),
            Scale::S15_000 => (
                (self.x * CONVERSION_15000).round(),
                -(self.y * CONVERSION_15000).round(),
            ),
        };

        if (x.abs() > MAX_MU) || (y.abs() > MAX_MU) {
            Err(OmapError::MapCoordinateOverflow)
        } else {
            Ok((x as i32, y as i32))
        }
    }
}
