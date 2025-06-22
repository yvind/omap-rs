#[derive(Debug, Clone, Copy)]
pub(crate) struct Transform {
    pub(crate) grivation: f64,
    scale_factor: f64,
    sin: f64,
    cos: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            grivation: 0.,
            scale_factor: 1.,
            sin: 0.,
            cos: 1.,
        }
    }
}

impl Transform {
    pub(crate) fn new(
        scale: crate::Scale,
        combined_scale_factor: f64,
        grivation: f64,
    ) -> Transform {
        Self {
            grivation,
            scale_factor: scale.get_map_scale_factor() / combined_scale_factor,
            sin: grivation.sin(),
            cos: grivation.cos(),
        }
    }

    pub(crate) fn world_to_map(&self, coord: geo_types::Coord) -> geo_types::Coord {
        let x = coord.x * self.cos - coord.y * self.sin;
        let y = coord.x * self.sin + coord.y * self.cos;

        geo_types::Coord {
            x: (x * self.scale_factor).round(),
            y: -(y * self.scale_factor).round(),
        }
    }
}
