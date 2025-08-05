use super::{MapObjectTrait, TagTrait};
use crate::writer::{
    serialize::{SerializeBezier, SerializePolyLine},
    symbols::{LineSymbol, SymbolTrait},
    transform::Transform,
    Result,
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

    /// change the symbol of a line object
    pub fn change_symbol(&mut self, symbol: LineSymbol) {
        self.symbol = symbol;
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
        transform: &Transform,
    ) -> Result<()> {
        f.write_all(format!("<object type=\"1\" symbol=\"{}\">", self.symbol.id()).as_bytes())?;
        self.write_tags(f)?;
        self.write_coords(f, bez_error, transform)?;
        f.write_all(b"</object>\n")?;
        Ok(())
    }

    fn write_coords(
        self,
        f: &mut BufWriter<File>,
        bez_error: Option<f64>,
        transform: &Transform,
    ) -> Result<()> {
        let (bytes, num_coords) = if let Some(bezier_error) = bez_error {
            self.line.serialize_bezier(bezier_error, transform)
        } else {
            self.line.serialize_polyline(transform)
        }?;
        f.write_all(format!("<coords count=\"{num_coords}\">").as_bytes())?;
        f.write_all(&bytes)?;
        f.write_all(b"</coords>")?;
        Ok(())
    }

    fn write_tags(&self, f: &mut BufWriter<File>) -> Result<()> {
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
