use std::collections::HashMap;

use geo_types::{Coord, LineString};
use linestring2bezier::{BezierCurve, BezierSegment, BezierString};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{FileCoord, PARSE_BEZIER_ERROR};
use crate::{
    Error, Result,
    symbols::{Symbol, SymbolSet, WeakLinePathSymbol},
    utils::{from_file_coords, to_file_coords, try_get_attr_raw},
};

/// A line object represented as a polyline on the map.
#[derive(Debug, Clone)]
pub struct LineObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    /// The line or combined-line symbol used to render this object.
    pub symbol: WeakLinePathSymbol,
    /// Whether the coordinates should be written back as bezier curves.
    pub write_as_bezier: bool,
    geometry: LineString,
    // store the raw map-file coords with flags so that the object can be written back unchanged if the coords are untouched
    // (so that the errors introduced when mapping from beziers to linestring and back only are introduced when necessary)
    raw_map_coords: Vec<FileCoord>,
    is_coords_touched: bool,
}

impl LineObject {
    /// Create a new line object with the given symbol and geometry.
    pub fn new(symbol: impl Into<WeakLinePathSymbol>, geometry: LineString) -> Self {
        LineObject {
            tags: HashMap::new(),
            symbol: symbol.into(),
            write_as_bezier: false,
            geometry,
            raw_map_coords: Vec::new(),
            is_coords_touched: true,
        }
    }

    /// Get a shared reference to the line geometry.
    pub fn get_geometry(&self) -> &LineString {
        &self.geometry
    }

    /// Get a mutable reference to the line geometry (marks coords as touched).
    pub fn get_geometry_mut(&mut self) -> &mut LineString {
        self.is_coords_touched = true;
        &mut self.geometry
    }

    /// Create a LineObject for use as a PointSymbol element (no map symbol needed)
    pub fn new_element(geometry: LineString) -> Self {
        LineObject {
            tags: HashMap::new(),
            symbol: WeakLinePathSymbol::Line(std::rc::Weak::new()),
            write_as_bezier: false,
            geometry,
            raw_map_coords: Vec::new(),
            is_coords_touched: true,
        }
    }

    /// Get coords for element writing
    pub fn get_element_coords(&self) -> impl Iterator<Item = &Coord<f64>> {
        self.geometry.coords()
    }

    /// Reverses a geometry and the input xml without marking it as touched
    pub fn reverse_linestring(&mut self) {
        self.geometry.0.reverse();
        self.raw_map_coords = reverse_raw_line_coords(&self.raw_map_coords);
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let idx = match &self.symbol {
            WeakLinePathSymbol::Line(weak) => {
                if let Some(sym) = weak.upgrade() {
                    symbol_set
                        .iter()
                        .position(|s| match s {
                            Symbol::Line(ref_cell) => ref_cell.as_ptr() == sym.as_ptr(),
                            _ => false,
                        })
                        .map(|p| p as i32)
                        .unwrap_or(-1)
                } else {
                    -1
                }
            }
            WeakLinePathSymbol::CombinedLine(weak) => {
                if let Some(sym) = weak.upgrade() {
                    symbol_set
                        .iter()
                        .position(|s| match s {
                            Symbol::CombinedLine(ref_cell) => ref_cell.as_ptr() == sym.as_ptr(),
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

    /// Write the object
    /// Uses raw coords if untouched, otherwise writes geometry
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
            if self.geometry.0.len() < 2 {
                return Err(Error::ObjectError);
            }
            self.write_geometry_coords(writer)?;
        }
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Write coords from the geometry, as bezier if self.write_as_bezier
    fn write_geometry_coords<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let content = if self.write_as_bezier {
            let bezier = BezierString::from_line_string(self.geometry.clone(), PARSE_BEZIER_ERROR)?;

            let num_coords = bezier.num_points();

            let bs = BytesStart::new("coords")
                .with_attributes([("count", num_coords.to_string().as_str())]);
            writer.write_event(Event::Start(bs))?;

            let mut content = String::with_capacity(num_coords * 18);

            let mut i = 0;
            while i < bezier.num_segments() - 1 {
                let current_segment = &bezier.0[i];
                match current_segment {
                    BezierSegment::Bezier(bezier_curve) => {
                        let s = to_file_coords(bezier_curve.start)?;
                        content.push_str(&s.x.to_string());
                        content.push(' ');
                        content.push_str(&s.y.to_string());
                        content.push_str(" 1;");

                        let h1 = to_file_coords(bezier_curve.handle1)?;
                        content.push_str(&h1.x.to_string());
                        content.push(' ');
                        content.push_str(&h1.y.to_string());
                        content.push(';');

                        let h2 = to_file_coords(bezier_curve.handle2)?;
                        content.push_str(&h2.x.to_string());
                        content.push(' ');
                        content.push_str(&h2.y.to_string());
                        content.push(';');
                    }
                    BezierSegment::Line(line) => {
                        let c = to_file_coords(line.start)?;
                        content.push_str(&c.x.to_string());
                        content.push(' ');
                        content.push_str(&c.y.to_string());
                        content.push(';');
                    }
                }
                i += 1;
            }
            let last_segment = &bezier.0[bezier.num_segments() - 1];
            match last_segment {
                BezierSegment::Bezier(bezier_curve) => {
                    let s = to_file_coords(bezier_curve.start)?;
                    content.push_str(&s.x.to_string());
                    content.push(' ');
                    content.push_str(&s.y.to_string());
                    content.push_str(" 1;");

                    let h1 = to_file_coords(bezier_curve.handle1)?;
                    content.push_str(&h1.x.to_string());
                    content.push(' ');
                    content.push_str(&h1.y.to_string());
                    content.push(';');

                    let h2 = to_file_coords(bezier_curve.handle2)?;
                    content.push_str(&h2.x.to_string());
                    content.push(' ');
                    content.push_str(&h2.y.to_string());
                    content.push(';');

                    let e = to_file_coords(bezier_curve.end)?;
                    content.push_str(&e.x.to_string());
                    content.push(' ');
                    content.push_str(&e.y.to_string());
                    if self.geometry.is_closed() {
                        content.push_str(" 18;");
                    } else {
                        content.push(';');
                    }
                }
                BezierSegment::Line(line) => {
                    let c = to_file_coords(line.start)?;
                    content.push_str(&c.x.to_string());
                    content.push(' ');
                    content.push_str(&c.y.to_string());
                    content.push(';');
                    let c = to_file_coords(line.end)?;
                    content.push_str(&c.x.to_string());
                    content.push(' ');
                    content.push_str(&c.y.to_string());
                    if self.geometry.is_closed() {
                        content.push_str(" 18;");
                    } else {
                        content.push(';');
                    }
                }
            }
            content
        } else {
            let num_coords = self.geometry.0.len();

            let bs = BytesStart::new("coords")
                .with_attributes([("count", num_coords.to_string().as_str())]);
            writer.write_event(Event::Start(bs))?;

            let mut content = String::with_capacity(num_coords * 16);
            for coord in self.geometry.coords() {
                let fc = to_file_coords(*coord)?;
                content.push_str(&fc.x.to_string());
                content.push(' ');
                content.push_str(&fc.y.to_string());
                content.push(';');
            }
            content
        };
        writer.write_event(Event::Text(BytesText::new(&content)))?;
        writer.write_event(Event::End(BytesEnd::new("coords")))?;
        Ok(())
    }

    /// Parse a line object. The reader should be positioned right after the `<object>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: WeakLinePathSymbol,
    ) -> Result<LineObject> {
        let mut tags = HashMap::new();
        let mut line = Vec::new();
        let mut raw_map_coords = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"coords" => {
                        let num_coords = try_get_attr_raw(&bytes_start, "count").unwrap_or(0);
                        line.reserve(num_coords);
                        raw_map_coords.reserve(num_coords);
                    }
                    b"tags" => tags = super::parse_tags(reader)?,
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

                        // for lines we only care about the bezier flag 1
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
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in LineObject parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }
        Ok(LineObject {
            tags,
            symbol,
            write_as_bezier: false,
            geometry: LineString::new(line),
            raw_map_coords,
            is_coords_touched: false,
        })
    }
}

pub(crate) fn reverse_raw_line_coords(coords: &[FileCoord]) -> Vec<FileCoord> {
    // iterate through and check the flags
    // flags 1, 2 and 16 must be moved
    // 2 and 16 can only exist at the end and must be moved there
    // flag 1 must be moved to the other end of the bezier
    let mut new_xml = Vec::with_capacity(coords.len());

    let mut end_flag = 0;
    for i in (0..coords.len()).rev() {
        let (coord, mut flag) = coords[i];
        // remove a possible bezier flag
        flag -= flag & 1;

        if i == coords.len() - 1 {
            end_flag += (flag & 2) + (flag & 16);
            flag -= end_flag;
        }
        if i > 2 {
            // check the flag of i + 3 for a 1
            let (_, bez_flag) = coords[i - 3];
            flag |= bez_flag & 1;
        } else if i == 0 {
            flag |= end_flag;
        }
        new_xml.push((coord, flag));
    }
    new_xml
}
