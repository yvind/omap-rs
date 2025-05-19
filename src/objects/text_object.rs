use crate::{
    objects::{MapObjectTrait, TagTrait},
    serialize::SerializePolyLine,
    symbols::{SymbolTrait, TextSymbol},
    OmapResult, Scale,
};
use geo_types::Point;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
};

/// A TextObject representing anything that has a TextSymbol
#[derive(Debug, Clone)]
pub struct TextObject {
    /// the coordinate (relative the ref point of the map)
    pub point: Point,
    /// the symbol
    pub symbol: TextSymbol,
    /// the actual text to display,
    pub text: String,
    /// tags for this object
    pub tags: HashMap<String, String>,
}

impl TextObject {
    /// create a text object from a geo_types::Point and String
    pub fn from_point(point: Point, symbol: TextSymbol, text: String) -> Self {
        Self {
            point,
            symbol,
            text,
            tags: HashMap::new(),
        }
    }
}

impl TagTrait for TextObject {
    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>) {
        let _ = self.tags.insert(k.into(), v.into());
    }
}

impl MapObjectTrait for TextObject {
    fn write_to_map(
        self,
        f: &mut BufWriter<File>,
        _as_bezier: Option<f64>,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<()> {
        f.write_all(
            format!(
                "<object type=\"4\" symbol=\"{}\" h_align=\"1\" v_align=\"2\">",
                self.symbol.id(),
            )
            .as_bytes(),
        )?;
        self.write_tags(f)?;
        let text = self.text.clone();
        self.write_coords(f, None, scale, grivation, inv_combined_scale_factor)?;
        f.write_all(format!("<text>{}</text>", text).as_bytes())?;
        f.write_all(b"</object>\n")?;
        Ok(())
    }

    fn write_coords(
        self,
        f: &mut BufWriter<File>,
        _as_bezier: Option<f64>,
        scale: Scale,
        grivation: f64,
        inv_combined_scale_factor: f64,
    ) -> OmapResult<()> {
        let (bytes, _) =
            self.point
                .serialize_polyline(scale, grivation, inv_combined_scale_factor)?;

        f.write_all(b"<coords count=\"1\">")?;
        f.write_all(&bytes)?;
        f.write_all(b"</coords>")?;
        Ok(())
    }

    fn write_tags(&self, f: &mut BufWriter<File>) -> OmapResult<()> {
        if self.tags.is_empty() {
            return Ok(());
        }

        f.write_all(b"<tags>")?;
        for (key, val) in self.tags.iter() {
            f.write_all(format!("<t k=\"{key}\">{val}</t>").as_bytes())?;
        }
        f.write_all(b"</tags>")?;
        Ok(())
    }
}
