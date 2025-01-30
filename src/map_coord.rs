use geo_types::Coord;

use crate::{OmapError, OmapResult, Scale};

pub(crate) trait MapCoord {
    fn to_map_coordinates(
        self,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<(i32, i32)>;
}

// 1 map unit is 0.001mm on paper => 1000 mu = 1mm on map = 15m on ground
const CONVERSION_15000: f64 = 1_000. / 15.;
const CONVERSION_10000: f64 = 1_000. / 10.;

const MAX_MU: f64 = 2147483647.; // 2^31 - 1 max number a i32 can hold

impl MapCoord for Coord {
    fn to_map_coordinates(
        self,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<(i32, i32)> {
        let sin = grivation.sin();
        let cos = grivation.cos();

        let x = self.x * cos - self.y * sin;
        let y = self.x * sin + self.y * cos;

        let (x, y) = match scale {
            Scale::S10_000 => (
                (x * CONVERSION_10000 * combined_scale_factor).round(),
                -(y * CONVERSION_10000 * combined_scale_factor).round(),
            ),
            Scale::S15_000 => (
                (x * CONVERSION_15000 * combined_scale_factor).round(),
                -(y * CONVERSION_15000 * combined_scale_factor).round(),
            ),
        };

        if (x.abs() > MAX_MU) || (y.abs() > MAX_MU) {
            Err(OmapError::MapCoordinateOverflow)
        } else {
            Ok((x as i32, y as i32))
        }
    }
}
