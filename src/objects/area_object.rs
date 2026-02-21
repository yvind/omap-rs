use std::collections::HashMap;

use geo_types::{Coord, LineString, Polygon};
use linestring2bezier::{BezierCurve, BezierSegment, BezierString};
use quick_xml::Writer;
use quick_xml::events::BytesStart;
use quick_xml::{Reader, events::Event};

use crate::objects::{MapCoord, PARSE_BEZIER_ERROR};
use crate::symbols::AreaObjectSymbol;
use crate::{Error, Result, parse_attr, try_get_attr};

#[derive(Debug, Clone, Default)]
pub struct PatternRotation {
    rotation: f64,
    coord: Coord,
}

#[derive(Debug, Clone)]
pub struct AreaObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    pub pattern_rotation: PatternRotation,
    pub symbol: AreaObjectSymbol,
    pub write_as_bezier: bool,
    geometry: Polygon,
    // store the raw map-file coords with flags so that the object can be written back unchanged if the coords are untouched
    // (so that the errors introduced when mapping from beziers to linestring and back only are introduced when necessary)
    raw_map_coords: Vec<MapCoord>,
    is_coords_touched: bool,
}

impl AreaObject {
    pub fn reverse_polygon(&mut self) {
        self.geometry.exterior_mut(|e| e.0.reverse());
        self.geometry
            .interiors_mut(|is| is.iter_mut().for_each(|i| i.0.reverse()));

        self.raw_map_coords = reverse_raw_polygon_coords(&self.raw_map_coords);
    }

    pub(super) fn write<W: std::io::Write>(&self, _writer: &mut Writer<W>) -> Result<()> {
        Ok(())
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
    ) -> Result<(Self, String)> {
        let mut raw_xml = String::new();
        let mut pr = PatternRotation::default();

        let mut num_coords = 0;

        for attr in element.attributes().filter_map(std::result::Result::ok) {
            if matches!(attr.key.local_name().as_ref(), b"count") {
                num_coords = parse_attr(attr.value).unwrap_or(num_coords);
            }
        }

        let mut linestrings = Vec::new();
        let mut line = Vec::with_capacity(num_coords);

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
                            pr.coord = Coord {
                                x: x as f64 / 1_000.,
                                y: -y as f64 / 1_000.,
                            }
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
                        "Unexpected EOF in LineObject parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }
        if !line.is_empty() {
            linestrings.push(LineString::new(line));
        }
        let exterior = linestrings.remove(0);
        Ok((
            AreaObject {
                polygon: Polygon::new(exterior, linestrings),
                pattern_rotation: pr,
            },
            raw_xml,
        ))
    }
}

fn reverse_raw_polygon_coords(coords: &[MapCoord]) -> Vec<MapCoord> {
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
