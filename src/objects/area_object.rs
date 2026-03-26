use std::collections::HashMap;

use geo_types::{Coord, LineString, Polygon};
use linestring2bezier::{BezierCurve, BezierSegment, BezierString};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{FileCoord, PARSE_BEZIER_ERROR};
use crate::{
    Error, Result,
    symbols::{Symbol, SymbolSet, WeakAreaPathSymbol},
    utils::{from_file_coords, to_file_coords, try_get_attr_raw},
};

/// A fill pattern rotation and origin used by area objects.
#[derive(Debug, Clone, Default)]
pub struct PatternRotation {
    /// Rotation of the fill pattern in radians.
    pub rotation: f64,
    /// Origin coordinate for the pattern.
    pub coord: Coord,
}

/// An area (polygon) object on the map.
#[derive(Debug, Clone)]
pub struct AreaObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    /// The fill-pattern rotation and origin.
    pub pattern_rotation: PatternRotation,
    /// The area or combined-area symbol used to render this object.
    pub symbol: WeakAreaPathSymbol,
    /// Whether the coordinates should be written back as bezier curves.
    pub write_as_bezier: bool,
    geometry: Polygon,
    // store the raw map-file coords with flags so that the object can be written back unchanged if the coords are untouched
    // (so that the errors introduced when mapping from beziers to linestring and back only are introduced when necessary)
    raw_map_coords: Vec<FileCoord>,
    is_coords_touched: bool,
}

impl AreaObject {
    /// Create a new area object with the given symbol and geometry.
    pub fn new(symbol: impl Into<WeakAreaPathSymbol>, geometry: Polygon) -> Self {
        AreaObject {
            tags: HashMap::new(),
            pattern_rotation: PatternRotation::default(),
            symbol: symbol.into(),
            write_as_bezier: false,
            geometry,
            raw_map_coords: Vec::new(),
            is_coords_touched: true,
        }
    }

    /// Get a shared reference to the polygon geometry.
    pub fn get_geometry(&self) -> &Polygon {
        &self.geometry
    }

    /// Get a mutable reference to the polygon geometry (marks coords as touched).
    pub fn get_geometry_mut(&mut self) -> &mut Polygon {
        self.is_coords_touched = true;
        &mut self.geometry
    }

    /// Reverse the winding order of all rings.
    pub fn reverse_polygon(&mut self) {
        self.geometry.exterior_mut(|e| e.0.reverse());
        self.geometry
            .interiors_mut(|is| is.iter_mut().for_each(|i| i.0.reverse()));

        self.raw_map_coords = reverse_raw_polygon_coords(&self.raw_map_coords);
    }

    /// Create an AreaObject for use as a PointSymbol element (no map symbol needed)
    pub fn new_element(geometry: Polygon) -> Self {
        AreaObject {
            tags: HashMap::new(),
            pattern_rotation: PatternRotation::default(),
            symbol: WeakAreaPathSymbol::Area(std::rc::Weak::new()),
            write_as_bezier: false,
            geometry,
            raw_map_coords: Vec::new(),
            is_coords_touched: true,
        }
    }

    /// Get coords for element writing (exterior ring coords)
    pub fn get_element_coords(&self) -> impl Iterator<Item = &Coord<f64>> {
        self.geometry.exterior().coords()
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let idx = match &self.symbol {
            WeakAreaPathSymbol::Area(weak) => {
                if let Some(sym) = weak.upgrade() {
                    symbol_set
                        .iter()
                        .position(|s| match s {
                            Symbol::Area(ref_cell) => ref_cell.as_ptr() == sym.as_ptr(),
                            _ => false,
                        })
                        .map(|p| p as i32)
                        .unwrap_or(-1)
                } else {
                    -1
                }
            }
            WeakAreaPathSymbol::CombinedArea(weak) => {
                if let Some(sym) = weak.upgrade() {
                    symbol_set
                        .iter()
                        .position(|s| match s {
                            Symbol::CombinedArea(ref_cell) => ref_cell.as_ptr() == sym.as_ptr(),
                            _ => false,
                        })
                        .map(|p| p as i32)
                        .unwrap_or(-1)
                } else {
                    -1
                }
            }
        };
        self.write_content(writer, Some(idx))?;
        Ok(())
    }

    /// Write a full `<object>...</object>` element - used for point symbol elements
    pub(crate) fn write_as_element<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        self.write_content(writer, None)?;
        Ok(())
    }

    /// Write the object.
    /// Uses raw coords if untouched, otherwise writes geometry.
    fn write_content<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_index: Option<i32>,
    ) -> Result<()> {
        let mut bs = BytesStart::new("object").with_attributes([("type", "1")]);
        if let Some(sid) = symbol_index {
            bs.push_attribute(("symbol", sid.to_string().as_str()));
        }
        writer.write_event(Event::Start(bs))?;
        // elements are not allowed to have tags
        if !self.tags.is_empty() && symbol_index.is_some() {
            super::write_tags(writer, &self.tags)?;
        }

        if !self.is_coords_touched && !self.raw_map_coords.is_empty() {
            super::write_raw_coords(writer, &self.raw_map_coords)?;
        } else {
            self.write_geometry_coords(writer)?;
        }
        self.write_pattern(writer)?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Write coords from the geometry, as bezier if self.write_as_bezier
    fn write_geometry_coords<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let mut all_coords: Vec<FileCoord> = Vec::new();

        // exterior ring
        let ext = self.geometry.exterior();
        for (i, coord) in ext.coords().enumerate() {
            let fc = to_file_coords(*coord)?;
            let flag = if i == ext.0.len() - 1 { 18_u8 } else { 0_u8 };
            all_coords.push((fc, flag));
        }
        // interior rings
        for interior in self.geometry.interiors() {
            for (i, coord) in interior.coords().enumerate() {
                let fc = to_file_coords(*coord)?;
                let flag = if i == interior.0.len() - 1 {
                    18_u8
                } else {
                    0_u8
                };
                all_coords.push((fc, flag));
            }
        }

        super::write_raw_coords(writer, &all_coords)?;
        Ok(())
    }

    /// Write the `<pattern>` element with the pattern rotation and origin coord
    fn write_pattern<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let pr = &self.pattern_rotation;
        let mut bs = BytesStart::new("pattern");
        bs.push_attribute(("rotation", pr.rotation.to_string().as_str()));
        writer.write_event(Event::Start(bs))?;
        let fc = to_file_coords(pr.coord)?;
        writer.write_event(Event::Empty(BytesStart::new("coord").with_attributes([
            ("x", fc.x.to_string().as_str()),
            ("y", fc.y.to_string().as_str()),
        ])))?;
        writer.write_event(Event::End(BytesEnd::new("pattern")))?;
        Ok(())
    }

    /// Parse an area object. The reader should be positioned right after the `<object>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: WeakAreaPathSymbol,
    ) -> Result<AreaObject> {
        let mut tags = HashMap::new();
        let mut pr = PatternRotation::default();
        let mut linestrings = Vec::new();
        let mut line = Vec::new();
        let mut raw_map_coords = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"coords" => {
                        let num_coords: usize =
                            try_get_attr_raw(&bytes_start, "count").unwrap_or(0);
                        line.reserve(num_coords);
                        raw_map_coords.reserve(num_coords);
                    }
                    b"pattern" => {
                        pr.rotation =
                            try_get_attr_raw(&bytes_start, "rotation").unwrap_or(pr.rotation)
                    }
                    b"tags" => tags = super::parse_tags(reader)?,
                    b"coord" => {
                        let x = try_get_attr_raw(&bytes_start, "x").unwrap_or(0);
                        let y = try_get_attr_raw(&bytes_start, "y").unwrap_or(0);
                        pr.coord = from_file_coords(Coord { x, y });
                    }
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    let raw_xml = str::from_utf8(bytes_text.as_ref())?;

                    let mut next_handle = 0_u8;
                    let mut bezier_buf = BezierString::empty();
                    let mut bezier_curve_buf = BezierCurve::zero();

                    for vertex in raw_xml.split_terminator(';') {
                        let mut parts: (i32, i32, u8) = (0, 0, 0);
                        let mut split = vertex.split_whitespace();

                        parts.0 = split
                            .next()
                            .ok_or(Error::InvalidCoordinate("No x value in split".to_string()))?
                            .parse()?;
                        parts.1 = split
                            .next()
                            .ok_or(Error::InvalidCoordinate("No y value in split".to_string()))?
                            .parse()?;
                        if let Some(e) = split.next() {
                            parts.2 = e.parse()?;
                        }

                        raw_map_coords.push((
                            Coord {
                                x: parts.0,
                                y: parts.1,
                            },
                            parts.2,
                        ));

                        let coord = from_file_coords(Coord {
                            x: parts.0,
                            y: parts.1,
                        });

                        // for areas we care about the bezier flag 1 and the close/hole flag 2
                        // check for start of bezier flag, and how far along a bezier we are
                        match (parts.2 & 1 == 1, next_handle) {
                            (true, 0) => {
                                // bezier start
                                bezier_curve_buf.start = coord;
                                next_handle += 1;
                            }
                            (true, 3) => {
                                // bezier end and next start
                                bezier_curve_buf.end = coord;
                                bezier_buf
                                    .0
                                    .push(BezierSegment::Bezier(bezier_curve_buf.clone()));
                                bezier_curve_buf.start = coord;
                                next_handle = 1;
                            }
                            (false, 1) => {
                                // bezier first handle
                                bezier_curve_buf.handle1 = coord;
                                next_handle += 1;
                            }
                            (false, 2) => {
                                // bezier second handle
                                bezier_curve_buf.handle2 = coord;
                                next_handle += 1;
                            }
                            (false, 3) => {
                                // end point
                                bezier_curve_buf.end = coord;
                                bezier_buf
                                    .0
                                    .push(BezierSegment::Bezier(bezier_curve_buf.clone()));

                                // convert the bezier to line string and add to end of line
                                line.extend(
                                    bezier_buf
                                        .clone()
                                        .to_line_string(PARSE_BEZIER_ERROR)?
                                        .into_inner(),
                                );
                                next_handle = 0;
                            }
                            (false, 0) => {
                                // normal coord
                                line.push(coord);
                            }
                            _ => return Err(Error::ObjectError),
                        }

                        // check for close/hole flag (flag & 2)
                        if (parts.2 & 2) == 2 {
                            linestrings.push(LineString::new(line));
                            line = Vec::new();
                        }
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in AreaObject parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }
        // Check if the polygon is not closed. Close it
        if !line.is_empty() {
            line.push(line[0]);
            linestrings.push(LineString::new(line));
        }
        let exterior = if linestrings.is_empty() {
            return Err(Error::ObjectError);
        } else {
            linestrings.remove(0)
        };
        Ok(AreaObject {
            tags,
            pattern_rotation: pr,
            symbol,
            write_as_bezier: false,
            geometry: Polygon::new(exterior, linestrings),
            raw_map_coords,
            is_coords_touched: false,
        })
    }
}

pub(crate) fn reverse_raw_polygon_coords(coords: &[FileCoord]) -> Vec<FileCoord> {
    // get each of the substrings for each loop and flip them
    // a substring ends with a 2 flag (often 18 or 50)
    let mut s = Vec::with_capacity(coords.len());
    let mut prev_split = 0;
    for (i, (_, f)) in coords.iter().enumerate() {
        if f & 2 == 2 || i == coords.len() - 1 {
            s.extend(crate::objects::line_object::reverse_raw_line_coords(
                &coords[prev_split..=i],
            ));
            prev_split = i + 1;
        }
    }
    s
}
