use std::{fs::File, io::BufWriter};

pub trait MapObject: 'static + Sync + Send {
    fn add_tag(&mut self, k: &str, v: &str);

    fn add_auto_tag(&mut self) {
        self.add_tag("auto-generated", "OmapMaker");
    }

    fn write_to_map(&self, f: &mut BufWriter<File>, bezier_error: Option<f64>);

    fn write_coords(&self, f: &mut BufWriter<File>, bezier_error: Option<f64>);

    fn write_tags(&self, f: &mut BufWriter<File>);
}
