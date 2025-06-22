use crate::{
    objects::{MapObjectTrait, TagTrait},
    serialize::{SerializeBezier, SerializePolyLine},
    symbols::{AreaSymbol, SymbolTrait},
    transform::Transform,
    OmapResult,
};
use geo_types::Polygon;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
};

/// A AreaObject representing anything that has a AreaSymbol
#[derive(Debug, Clone)]
pub struct AreaObject {
    /// the polygon with coordinates relative the maps ref-point
    pub polygon: Polygon,
    /// any area_symbol
    pub symbol: AreaSymbol,
    /// some area symbols have a rotation on the pattern  
    /// this field is only respected if `symbol.is_rotatable()`
    pub pattern_rotation: f64,
    /// tags for the object
    pub tags: HashMap<String, String>,
}

impl AreaObject {
    /// create an area object from a geo_types::Polygon
    pub fn from_polygon(polygon: Polygon, symbol: AreaSymbol, pattern_rotation: f64) -> Self {
        Self {
            polygon,
            symbol,
            pattern_rotation,
            tags: HashMap::new(),
        }
    }
}

impl TagTrait for AreaObject {
    fn add_tag(&mut self, k: impl Into<String>, v: impl Into<String>) {
        let _ = self.tags.insert(k.into(), v.into());
    }
}

impl MapObjectTrait for AreaObject {
    fn write_to_map(
        self,
        f: &mut BufWriter<File>,
        bez_error: Option<f64>,
        transform: &Transform,
    ) -> OmapResult<()> {
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
    ) -> OmapResult<()> {
        let (bytes, num_coords) = if let Some(bezier_error) = bez_error {
            self.polygon.serialize_bezier(bezier_error, transform)
        } else {
            self.polygon.serialize_polyline(transform)
        }?;
        f.write_all(format!("<coords count=\"{num_coords}\">").as_bytes())?;
        f.write_all(&bytes)?;
        f.write_all(b"</coords>")?;
        if self.symbol.is_rotatable() {
            f.write_all(
                format!(
                    "<pattern rotation=\"{}\"><coord x=\"0\" y=\"0\"/></pattern>",
                    self.pattern_rotation + transform.grivation
                )
                .as_bytes(),
            )?;
        }
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
