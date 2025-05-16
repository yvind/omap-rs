use crate::{
    map_coord::MapCoord,
    map_object::MapObjectTrait,
    symbol::{AreaSymbol, SymbolTrait},
    OmapResult, Scale, TagTrait,
};
use geo_types::Polygon;
use linestring2bezier::{BezierSegment, BezierString};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
};

/// A AreaObject representing anything that has a AreaSymbol
#[derive(Debug, Clone)]
pub struct AreaObject {
    /// the polygon with coordinates relative the maps refpoint
    pub polygon: Polygon,
    /// any areasymbol
    pub symbol: AreaSymbol,
    // some area symbols have a rotation on the pattern
    // pub rotation: f64,
    /// tags for the object
    pub tags: HashMap<String, String>,
}

impl AreaObject {
    /// create an area object from a geo_types::Polygon
    pub fn from_polygon(polygon: Polygon, symbol: AreaSymbol) -> Self {
        Self {
            polygon,
            symbol,
            tags: HashMap::new(),
        }
    }

    fn write_polyline(
        self,
        f: &mut BufWriter<File>,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()> {
        let coordinates = self.polygon;

        let mut num_coords = coordinates.exterior().0.len();
        let boundary_length = num_coords;

        for hole in coordinates.interiors().iter() {
            num_coords += hole.0.len();
        }

        f.write_all(format!("<coords count=\"{}\">", num_coords).as_bytes())?;

        let mut ext_iter = coordinates.exterior().coords();
        let mut i = 0;

        while i < boundary_length - 1 {
            let c = ext_iter.next().unwrap().to_map_coordinates(
                scale,
                grivation,
                combined_scale_factor,
            )?;
            f.write_all(format!("{} {};", c.0, c.1).as_bytes())?;
            i += 1;
        }
        let c =
            ext_iter
                .next()
                .unwrap()
                .to_map_coordinates(scale, grivation, combined_scale_factor)?;
        f.write_all(format!("{} {} 18;", c.0, c.1).as_bytes())?;

        for hole in coordinates.interiors().iter() {
            let hole_length = hole.0.len();

            let mut int_iter = hole.coords();
            let mut i = 0;

            while i < hole_length - 1 {
                let c = int_iter.next().unwrap().to_map_coordinates(
                    scale,
                    grivation,
                    combined_scale_factor,
                )?;
                f.write_all(format!("{} {};", c.0, c.1).as_bytes())?;

                i += 1;
            }
            let c = int_iter.next().unwrap().to_map_coordinates(
                scale,
                grivation,
                combined_scale_factor,
            )?;
            f.write_all(format!("{} {} 18;", c.0, c.1).as_bytes())?;
        }
        f.write_all(b"</coords>")?;

        Ok(())
    }

    fn write_bezier(
        self,
        f: &mut BufWriter<File>,
        error: f64,
        scale: Scale,
        grivation: f64,
        combined_scale_factor: f64,
    ) -> OmapResult<()> {
        let coordinates = self.polygon;

        let mut beziers = Vec::with_capacity(coordinates.num_rings());

        let (exterior, interiors) = coordinates.into_inner();

        beziers.push(BezierString::from_linestring(exterior, error));
        for hole in interiors {
            beziers.push(BezierString::from_linestring(hole, error));
        }
        let mut num_coords = 0;
        for b in beziers.iter() {
            num_coords += b.num_points();
        }

        f.write_all(format!("<coords count=\"{num_coords}\">").as_bytes())?;

        for bezier in beziers {
            let num_segments = bezier.0.len();

            let mut bez_iterator = bezier.0.into_iter();
            let mut i = 0;
            while i < num_segments - 1 {
                let segment = bez_iterator.next().unwrap();

                let BezierSegment {
                    start,
                    handles,
                    end: _,
                } = segment;

                if let Some(handles) = handles {
                    let c = start.to_map_coordinates(scale, grivation, combined_scale_factor)?;
                    let h1 =
                        handles
                            .0
                            .to_map_coordinates(scale, grivation, combined_scale_factor)?;
                    let h2 =
                        handles
                            .1
                            .to_map_coordinates(scale, grivation, combined_scale_factor)?;
                    f.write_all(
                        format!("{} {} 1;{} {};{} {};", c.0, c.1, h1.0, h1.1, h2.0, h2.1)
                            .as_bytes(),
                    )?;
                } else {
                    let c = start.to_map_coordinates(scale, grivation, combined_scale_factor)?;

                    f.write_all(format!("{} {};", c.0, c.1).as_bytes())?;
                }
                i += 1;
            }
            // finish with the last segment of the curve
            let final_segment = bez_iterator.next().unwrap();

            let BezierSegment {
                start,
                handles,
                end,
            } = final_segment;

            if let Some(handles) = handles {
                let c1 = start.to_map_coordinates(scale, grivation, combined_scale_factor)?;
                let h1 = handles
                    .0
                    .to_map_coordinates(scale, grivation, combined_scale_factor)?;
                let h2 = handles
                    .1
                    .to_map_coordinates(scale, grivation, combined_scale_factor)?;
                let c2 = end.to_map_coordinates(scale, grivation, combined_scale_factor)?;

                f.write_all(
                    format!(
                        "{} {} 1;{} {};{} {};{} {} 18;",
                        c1.0, c1.1, h1.0, h1.1, h2.0, h2.1, c2.0, c2.1
                    )
                    .as_bytes(),
                )?;
            } else {
                let c1 = start.to_map_coordinates(scale, grivation, combined_scale_factor)?;
                let c2 = end.to_map_coordinates(scale, grivation, combined_scale_factor)?;

                f.write_all(format!("{} {};{} {} 18;", c1.0, c1.1, c2.0, c2.1).as_bytes())?;
            }
        }
        f.write_all(b"</coords>")?;
        Ok(())
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
        if let Some(error) = bez_error {
            self.write_bezier(f, error, scale, grivation, combined_scale_factor)
        } else {
            self.write_polyline(f, scale, grivation, combined_scale_factor)
        }
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
