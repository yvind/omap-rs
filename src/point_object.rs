use crate::{map_geo_traits::MapCoord, map_object::MapObjectTrait, OmapResult, Scale, Symbol, Tag};
use geo_types::Point;

use std::{
    fs::File,
    io::{BufWriter, Write},
};

pub struct PointObject {
    symbol: Symbol,
    tags: Vec<Tag>,
}

impl PointObject {
    pub fn from_symbol(symbol: Symbol) -> Self {
        Self {
            symbol,
            tags: vec![],
        }
    }
}

impl MapObjectTrait for PointObject {
    fn add_tag(&mut self, k: &str, v: &str) {
        self.tags.push(Tag::new(k, v));
    }

    fn write_to_map(
        self,
        f: &mut BufWriter<File>,
        _as_bezier: Option<f64>,
        scale: Scale,
        grivation: f32,
    ) -> OmapResult<()> {
        f.write_all(
            format!(
                "<object type=\"0\" symbol=\"{}\" rotation=\"{}\">",
                self.symbol.id(),
                self.symbol.rotation()
            )
            .as_bytes(),
        )?;
        self.write_tags(f)?;
        self.write_coords(f, None, scale, grivation)?;
        f.write_all(b"</object>\n")?;
        Ok(())
    }

    fn write_coords(
        self,
        f: &mut BufWriter<File>,
        _as_bezier: Option<f64>,
        scale: Scale,
        grivation: f32,
    ) -> OmapResult<()> {
        let c = Point::try_from(self.symbol)
            .unwrap()
            .0
            .to_map_coordinates(scale, grivation)?;
        f.write_all(format!("<coords count=\"1\">{} {};</coords>", c.0, c.1).as_bytes())?;
        Ok(())
    }

    fn write_tags(&self, f: &mut BufWriter<File>) -> OmapResult<()> {
        if self.tags.is_empty() {
            return Ok(());
        }

        f.write_all(b"<tags>")?;
        for tag in self.tags.iter() {
            f.write_all(tag.to_string().as_bytes())?;
        }
        f.write_all(b"</tags>")?;
        Ok(())
    }
}
