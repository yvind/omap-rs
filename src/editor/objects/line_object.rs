use geo_types::{Coord, LineString};
use linestring2bezier::BezierSegment;
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

use crate::editor::{Error, Result};

use super::PatternRotation;

#[derive(Debug, Clone)]
pub struct LineObject {
    pub line: LineString,
    pub pattern_rotation: PatternRotation,
}

impl LineObject {
    pub(super) fn write<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        Ok(())
    }
}

impl LineObject {
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart,
    ) -> Result<(Self, String)> {
        let mut raw_xml = String::new();
        let mut pr = PatternRotation::default();

        let mut num_coords = 0;

        for attr in element.attributes() {
            let attr = attr?;

            if matches!(attr.key.local_name().as_ref(), b"count") {
                num_coords = std::str::from_utf8(&attr.value)?.parse()?;
            }
        }

        let mut line = Vec::with_capacity(num_coords);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"pattern") {
                        for attr in bytes_start.attributes() {
                            let attr = attr?;

                            if matches!(attr.key.local_name().as_ref(), b"rotation") {
                                pr.rotation = std::str::from_utf8(&attr.value)?.parse()?;
                            }
                        }
                    }
                }
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Empty(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"coord") {
                        for attr in bytes_start.attributes() {
                            let attr = attr?;

                            let mut x = None;
                            let mut y = None;
                            match attr.key.local_name().as_ref() {
                                b"x" => x = Some(std::str::from_utf8(&attr.value)?.parse::<i32>()?),
                                b"y" => y = Some(std::str::from_utf8(&attr.value)?.parse::<i32>()?),
                                _ => (),
                            }

                            if x.is_some() && y.is_some() {
                                pr.coord = Coord {
                                    x: x.unwrap() as f64 / 1_000.,
                                    y: -y.unwrap() as f64 / 1_000.,
                                }
                            }
                        }
                    }
                }
                Event::Text(bytes_text) => {
                    raw_xml = String::from_utf8(bytes_text.to_vec())?;

                    let mut bezier_state = 0_u8;

                    let mut bezier_buf: [Coord; 4] = Default::default();
                    let mut bezier_keep_first = true;

                    for vertex in raw_xml.split(';') {
                        let mut parts: (i32, i32, u8) = (0, 0, 0);
                        let mut split = vertex.split(' ');

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

                        // check flags, we only care about beziers for lines
                        //  1 = Bezier curve start
                        if (parts.2 & 1) == 1 && bezier_state == 0 {
                            // bezier start
                            bezier_keep_first = true;

                            bezier_buf[0] = coord;
                            bezier_state = 1;
                        } else if (parts.2 & 1) == 1 && bezier_state == 3 {
                            // bezier end and next start
                            bezier_keep_first = false;

                            bezier_buf[3] = coord;

                            // convert the bezier to line string and add to end of line
                            line.extend(
                                BezierSegment::from(bezier_buf)
                                    .to_line_string(density, bezier_keep_first)
                                    .into_inner(),
                            );

                            bezier_buf[0] = coord;

                            bezier_state = 1;
                        } else if bezier_state == 1 {
                            // first handle
                            bezier_buf[1] = coord;
                            bezier_state = 2;
                        } else if bezier_state == 2 {
                            // second handle
                            bezier_buf[2] = coord;
                            bezier_state = 3;
                        } else if bezier_state == 3 {
                            // end point
                            bezier_buf[3] = coord;

                            // convert the bezier to line string and add to end of line
                            line.extend(
                                BezierSegment::from(bezier_buf)
                                    .to_line_string(density, bezier_keep_first)
                                    .into_inner(),
                            );

                            bezier_state = 0;
                        } else {
                            // normal coord
                            line.push(coord);
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
                pattern_rotation: pr,
            },
            raw_xml,
        ))
    }
}
