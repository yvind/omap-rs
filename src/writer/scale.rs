/// Map scale
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Scale {
    /// 1:10_000
    S10_000,
    /// 1:15_000
    #[default]
    S15_000,
}

use std::fmt;
impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scale::S10_000 => write!(f, "10000"),
            Scale::S15_000 => write!(f, "15000"),
        }
    }
}

// 1 map unit is 0.001mm on paper => 1000 mu = 1mm on map
const CONVERSION_15000: f64 = 1_000. / 15.;
const CONVERSION_10000: f64 = 1_000. / 10.;
impl Scale {
    /// Get scale factor for converting ground distances to map units
    pub(crate) fn get_map_scale_factor(&self) -> f64 {
        match self {
            Scale::S10_000 => CONVERSION_10000,
            Scale::S15_000 => CONVERSION_15000,
        }
    }
}
