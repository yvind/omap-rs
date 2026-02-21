use std::collections::HashMap;

use geo_types::{Coord, LineString};
use linestring2bezier::{BezierCurve, BezierSegment, BezierString};
use quick_xml::{
    Reader, Writer,
    events::{BytesStart, Event},
};

use super::{MapCoord, PARSE_BEZIER_ERROR};
use crate::{Error, Result, symbols::LineObjectSymbol, try_get_attr};

#[derive(Debug, Clone)]
pub struct LineObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    pub symbol: LineObjectSymbol,
    pub write_as_bezier: bool,
    geometry: LineString,
    // store the raw map-file coords with flags so that the object can be written back unchanged if the coords are untouched
    // (so that the errors introduced when mapping from beziers to linestring and back only are introduced when necessary)
    raw_map_coords: Vec<MapCoord>,
    is_coords_touched: bool,
}

impl LineObject {
    pub fn get_geometry(&self) -> &LineString {
        &self.geometry
    }

    pub fn get_geometry_mut(&mut self) -> &mut LineString {
        self.is_coords_touched = true;
        &mut self.geometry
    }

    /// Reverses a geometry and the input xml without marking it as touched
    pub fn reverse_linestring(&mut self) {
        self.geometry.0.reverse();
        reverse_raw_line_coords(&mut self.raw_map_coords);
    }

    pub(super) fn write<W: std::io::Write>(self, _writer: &mut Writer<W>) -> Result<()> {
        Ok(())
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
    ) -> Result<(Self, String)> {
        let mut raw_xml = String::new();

        let mut num_coords = try_get_attr(&element, "count").unwrap_or(0);

        let mut line = Vec::with_capacity(num_coords);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    raw_xml = String::from_utf8(bytes_text.to_vec())?;

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

                        let coord = Coord {
                            x: parts.0 as f64 / 1_000.,
                            y: -parts.1 as f64 / 1_000.,
                        };

                        // check flags, we only care about bezier flags for lines
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
        Ok((
            LineObject {
                line: LineString::new(line),
            },
            raw_xml,
        ))
    }
}

pub(crate) fn reverse_raw_line_coords(coords: &[MapCoord]) -> Vec<MapCoord> {
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
