use crate::{OmapResult, Scale};
use std::{fs::File, io::BufWriter};

mod area_object;
mod line_object;
mod map_object;
mod point_object;
mod text_object;

pub use area_object::AreaObject;
pub use line_object::LineObject;
pub use map_object::MapObject;
pub use point_object::PointObject;
pub use text_object::TextObject;

pub(crate) trait MapObjectTrait {
    fn write_to_map(
        self,
        f: &mut BufWriter<File>,
        bezier_error: Option<f64>,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()>;

    fn write_coords(
        self,
        f: &mut BufWriter<File>,
        bezier_error: Option<f64>,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()>;

    fn write_tags(&self, f: &mut BufWriter<File>) -> OmapResult<()>;
}

/// trait for adding tags to objects
pub trait TagTrait {
    /// add any tag
    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>);

    /// add an elevation tag
    fn add_elevation_tag(&mut self, elevation: f64) {
        self.add_tag("Elevation", format!("{:.2}", elevation));
    }
}
