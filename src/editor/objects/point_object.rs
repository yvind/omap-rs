use geo_types::{Coord, Point};
use quick_xml::{Reader, events::Event};

use crate::editor::{Error, Result};

#[derive(Debug, Clone)]
pub struct PointObject {
    pub point: Point,
    pub rotation: f64,
}

impl PointObject {
    pub(super) fn get_special_keys(&self) -> Option<String> {
        Some(format!("rotation=\"{}\"", self.rotation))
    }

    pub(super) fn write<W: std::io::Write>(self, _writer: &mut W) -> Result<()> {
        todo!();
        // let map_coords = transform.to_map_coords(self.point.0);
        // writer.write_all(
        //     format!(
        //         "<coords count=\"1\">{} {};</coords>",
        //         map_coords.0, map_coords.1
        //     )
        //     .as_bytes(),
        // )?;
        // Ok(())
    }
}

impl PointObject {
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        rotation: f64,
    ) -> Result<Self> {
        let mut point = None;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    let raw_xml = String::from_utf8(bytes_text.to_vec())?;

                    for vertex in raw_xml.split(';') {
                        if vertex.is_empty() {
                            continue;
                        }
                        let mut parts: (i32, i32) = (0, 0);
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

                        let coord = Coord {
                            x: parts.0 as f64 / 1_000.,
                            y: -parts.1 as f64 / 1_000.,
                        };
                        point = Some(Point::from(coord));
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
        Ok(PointObject {
            point: point.ok_or(Error::ParseOmapFileError(
                "Could not parse point object".to_string(),
            ))?,
            rotation,
        })
    }
}
