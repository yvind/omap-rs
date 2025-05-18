use crate::{
    geometry::Serialize,
    objects::{MapObjectTrait, TagTrait},
    symbols::{LineSymbol, SymbolTrait},
    OmapResult, Scale,
};
use geo_types::LineString;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
};

/// A LineObject representing anything that has a LineSymbol
#[derive(Debug, Clone)]
pub struct LineObject {
    /// the linestring with coordinates relative the maps ref-point
    pub line: LineString,
    /// any line symbol
    pub symbol: LineSymbol,
    /// tags for the object
    pub tags: HashMap<String, String>,
}

impl LineObject {
    /// create a line object from a geo_types::LineString
    pub fn from_line_string(line: LineString, symbol: LineSymbol) -> Self {
        Self {
            line,
            symbol,
            tags: HashMap::new(),
        }
    }
}

impl TagTrait for LineObject {
    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>) {
        let _ = self.tags.insert(k.into(), v.into());
    }
}

impl MapObjectTrait for LineObject {
    fn write_to_map(
        self,
        f: &mut BufWriter<File>,
        bez_error: Option<f64>,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()> {
        f.write_all(format!("<object type=\"1\" symbol=\"{}\">", self.symbol.id()).as_bytes())?;
        self.write_tags(f)?;
        self.write_coords(f, bez_error, scale, grivation, combined_scale_factor)?;
        f.write_all(b"</object>\n")?;
        Ok(())
    }

    fn write_coords(
        self,
        f: &mut BufWriter<File>,
        bez_error: Option<f64>,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()> {
        let (bytes, num_coords) = if let Some(bezier_error) = bez_error {
            self.line
                .serialize_bezier(bezier_error, scale, grivation, combined_scale_factor)
        } else {
            self.line
                .serialize_polyline(scale, grivation, combined_scale_factor)
        }?;
        f.write_all(format!("<coords count=\"{num_coords}\">").as_bytes())?;
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
