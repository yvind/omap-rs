use std::collections::HashMap;

use geo_types::{Coord, LineString, Polygon};
use linestring2bezier::{BezierCurve, BezierSegment, BezierString};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use crate::objects::{MapCoord, PARSE_BEZIER_ERROR, write_raw_coords};
use crate::symbols::AreaObjectSymbol;
use crate::{
    Error, Result,
    utils::{from_file_coords, to_file_coords, try_get_attr},
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
    pub symbol: AreaObjectSymbol,
    /// Whether the coordinates should be written back as bezier curves.
    pub write_as_bezier: bool,
    geometry: Polygon,
    // store the raw map-file coords with flags so that the object can be written back unchanged if the coords are untouched
    // (so that the errors introduced when mapping from beziers to linestring and back only are introduced when necessary)
    raw_map_coords: Vec<MapCoord>,
    is_coords_touched: bool,
}

impl AreaObject {
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
            symbol: AreaObjectSymbol::Area(std::rc::Weak::new()),
            write_as_bezier: false,
            geometry,
            raw_map_coords: Vec::new(),
            is_coords_touched: false,
        }
    }

    /// Get coords for element writing (exterior ring coords)
    pub fn get_element_coords(&self) -> impl Iterator<Item = &Coord<f64>> {
        self.geometry.exterior().coords()
    }

    /// Write just the inner content (coords + pattern) — called from MapObject::write.
    /// Uses raw coords if untouched, otherwise converts geometry to file coords.
    pub(super) fn write_content<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        if !self.is_coords_touched && !self.raw_map_coords.is_empty() {
            write_raw_coords(writer, &self.raw_map_coords)?;
        } else {
            self.write_geometry_coords(writer)?;
        }
        self.write_pattern(writer)?;
        Ok(())
    }

    /// Write a full `<object>...</object>` element — used for point symbol elements
    pub fn write_as_element<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let bs = BytesStart::new("object").with_attributes([("type", "1")]);
        writer.write_event(Event::Start(bs))?;
        self.write_element_coords(writer)?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Write coords from the geometry (for map objects, with all rings and proper flags)
    fn write_geometry_coords<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let mut all_coords: Vec<MapCoord> = Vec::new();

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

        write_raw_coords(writer, &all_coords)?;
        Ok(())
    }

    /// Write coords from geometry for element objects (just exterior, with close flag)
    fn write_element_coords<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let ext = self.geometry.exterior();
        let coords: Vec<_> = ext.coords().collect();
        let bs = BytesStart::new("coords")
            .with_attributes([("count", coords.len().to_string().as_str())]);
        writer.write_event(Event::Start(bs))?;
        let mut content = String::new();
        for (i, coord) in coords.iter().enumerate() {
            let fc = to_file_coords(**coord)?;
            content.push_str(&fc.x.to_string());
            content.push(' ');
            content.push_str(&fc.y.to_string());
            if i == coords.len() - 1 {
                content.push_str(" 18");
            }
            content.push(';');
        }
        writer.write_event(Event::Text(BytesText::new(&content)))?;
        writer.write_event(Event::End(BytesEnd::new("coords")))?;
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

    /// Parse an area object. The reader should be positioned right after
    /// the `<coords>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        coords_element: &BytesStart<'_>,
    ) -> Result<AreaObject> {
        let mut pr = PatternRotation::default();
        let num_coords: usize = try_get_attr(coords_element, "count").unwrap_or(0);

        let mut linestrings = Vec::new();
        let mut line = Vec::with_capacity(num_coords);
        let mut raw_map_coords = Vec::with_capacity(num_coords);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"pattern" => {
                        pr.rotation = try_get_attr(&bytes_start, "rotation").unwrap_or(pr.rotation)
                    }
                    b"coord" => {
                        let x = try_get_attr::<i32>(&bytes_start, "x");
                        let y = try_get_attr::<i32>(&bytes_start, "y");

                        if let Some(x) = x
                            && let Some(y) = y
                        {
                            pr.coord = from_file_coords(Coord { x, y });
                        }
                    }
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    let raw_xml = String::from_utf8(bytes_text.to_vec())?;

                    let mut handles_written = 0_u8;
                    let mut bezier_on = false;
                    let mut bezier_buf = BezierString::empty();
                    let mut bezier_curve_buf = BezierCurve::zero();

                    for vertex in raw_xml.split_terminator(';') {
                        let mut parts: (i32, i32, u8) = (0, 0, 0);
                        let mut split = vertex.split_whitespace();

                        if let Some(e) = split.next() {
                            parts.0 = e.parse()?;
                        } else {
                            return Err(Error::InvalidCoordinate(
                                "No x value in split".to_string(),
                            ));
                        }
                        if let Some(e) = split.next() {
                            parts.1 = e.parse()?;
                        } else {
                            return Err(Error::InvalidCoordinate(
                                "No y value in split".to_string(),
                            ));
                        }
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

                        // check flags, we only care about the bezier flag '1' and the end flag '18'
                        //  1 = Bezier curve start
                        if (parts.2 & 1) == 1 && !bezier_on {
                            // bezier start
                            bezier_curve_buf.start = coord;
                            bezier_on = true;
                        } else if (parts.2 & 1) == 1 && bezier_on {
                            // bezier end and next start
                            bezier_curve_buf.end = coord;
                            bezier_buf
                                .0
                                .push(BezierSegment::Bezier(bezier_curve_buf.clone()));
                            bezier_curve_buf.start = coord;
                        } else if bezier_on && handles_written < 2 {
                            // first handles
                            if handles_written == 0 {
                                bezier_curve_buf.handle1 = coord;
                            } else if handles_written == 1 {
                                bezier_curve_buf.handle2 = coord;
                            }
                            handles_written += 1;
                        } else if bezier_on && handles_written == 2 {
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

                            bezier_on = false;
                            handles_written = 0;
                        } else if !bezier_on {
                            // normal coord
                            line.push(coord);
                        } else {
                            debug_assert!(false, "This should not be reachable in line parsing")
                        }

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
        if !line.is_empty() {
            linestrings.push(LineString::new(line));
        }
        let exterior = if linestrings.is_empty() {
            LineString::new(vec![])
        } else {
            linestrings.remove(0)
        };
        Ok(AreaObject {
            tags: HashMap::new(),
            pattern_rotation: pr,
            symbol: AreaObjectSymbol::Area(std::rc::Weak::new()),
            write_as_bezier: false,
            geometry: Polygon::new(exterior, linestrings),
            raw_map_coords,
            is_coords_touched: false,
        })
    }
}

pub(crate) fn reverse_raw_polygon_coords(coords: &[MapCoord]) -> Vec<MapCoord> {
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
