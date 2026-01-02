use crate::editor::{Error, Result};
use geo_types::{Coord, Point};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

#[derive(Debug, Clone)]
pub enum TextGeometry {
    SingleAnchor(Point),
    WrapBox(WrapBox),
}

#[derive(Debug, Clone)]
pub struct WrapBox {
    anchor: Point,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    Left = 0,
    HCenter = 1,
    Right = 2,
}

impl HorizontalAlign {
    pub(super) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"0" => Some(HorizontalAlign::Left),
            b"1" => Some(HorizontalAlign::HCenter),
            b"2" => Some(HorizontalAlign::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    Baseline = 0,
    Top = 1,
    VCenter = 2,
    Bottom = 3,
}

impl VerticalAlign {
    pub(super) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"0" => Some(VerticalAlign::Baseline),
            b"1" => Some(VerticalAlign::Top),
            b"2" => Some(VerticalAlign::VCenter),
            b"3" => Some(VerticalAlign::Bottom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextObject {
    pub geometry: TextGeometry,
    pub text: String,
    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
    pub rotation: f64,
}

impl TextObject {
    pub(crate) fn get_special_keys(&self) -> Option<String> {
        if self.rotation.is_normal() {
            Some(format!(
                "rotation=\"{}\" h_align=\"{}\" v_align=\"{}\"",
                self.rotation, self.h_align as u8, self.v_align as u8
            ))
        } else {
            Some(format!(
                "h_align=\"{}\" v_align=\"{}\"",
                self.h_align as u8, self.v_align as u8
            ))
        }
    }

    pub(crate) fn write<W: std::io::Write>(self, _writer: &mut W) -> Result<()> {
        todo!();
        //let coords_tag = match self.geometry {
        //    TextGeometry::SingleAnchor(p) => {
        //        let map_coords = transform.to_map_coords(p.0);
        //        format!(
        //            "<coords count=\"1\">{} {};</coords>",
        //            map_coords.0, map_coords.1
        //        )
        //    }
        //    TextGeometry::WrapBox(wp) => {
        //        let map_coords = transform.to_map_coords(wp.anchor.0);
        //        let width = transform.to_map_dist(wp.width);
        //        let height = transform.to_map_dist(wp.height);
        //
        //        format!(
        //            "<coords count=\"2\">{} {};{} {};</coords><size width=\"{}\" height=\"{}\"/>",
        //            map_coords.0, map_coords.1, width, height, width, height
        //        )
        //    }
        //};
        //
        //writer.write_all(format!("{}<text>{}</text>", coords_tag, self.text).as_bytes())?;
        //Ok(())
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        h_align: Option<HorizontalAlign>,
        v_align: Option<VerticalAlign>,
        rotation: f64,
        element: &BytesStart<'_>,
    ) -> Result<Self> {
        let mut num_coords = 0;
        for attr in element.attributes().filter_map(std::result::Result::ok) {
            if matches!(attr.key.local_name().as_ref(), b"count") {
                num_coords = std::str::from_utf8(&attr.value)?.parse()?;
            }
        }
        if num_coords == 0 {
            return Err(Error::ParseOmapFileError("".to_string()));
        }

        let mut text_geo = if num_coords == 1 {
            TextGeometry::SingleAnchor(Point::default())
        } else {
            TextGeometry::WrapBox(WrapBox {
                anchor: Point::default(),
                width: 0.,
                height: 0.,
            })
        };
        let mut is_coords_read = false;
        let mut text = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    match is_coords_read {
                        false => {
                            // parse the text location
                            is_coords_read = true;

                            let raw_xml = String::from_utf8(bytes_text.to_vec())?;

                            if let Some((coords, opt_wh)) = raw_xml.split_once(';') {
                                let mut parts: (i32, i32) = (0, 0);
                                let mut split = coords.split_whitespace();

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

                                let (width, height) = if !opt_wh.is_empty() {
                                    let mut split = coords.split_whitespace();

                                    let width = if let Some(e) = split.next() {
                                        e.parse()?
                                    } else {
                                        return Err(Error::InvalidCoordinate(
                                            "No x value in split".to_string(),
                                        ));
                                    };
                                    let height = if let Some(e) = split.next() {
                                        e.parse()?
                                    } else {
                                        return Err(Error::InvalidCoordinate(
                                            "No y value in split".to_string(),
                                        ));
                                    };
                                    (Some(width), Some(height))
                                } else {
                                    (None, None)
                                };

                                match &mut text_geo {
                                    TextGeometry::SingleAnchor(point) => {
                                        *point = Point::from(coord)
                                    }
                                    TextGeometry::WrapBox(wrap_box) => {
                                        wrap_box.anchor = Point::from(coord);
                                        wrap_box.width = width.ok_or(Error::ParseOmapFileError(
                                            "Could not parse text symbol in wrap box".to_string(),
                                        ))?;
                                        wrap_box.height =
                                            height.ok_or(Error::ParseOmapFileError(
                                                "Could not parse text symbol in wrap box"
                                                    .to_string(),
                                            ))?;
                                    }
                                };
                            } else {
                                return Err(Error::ParseOmapFileError(
                                    "Could not parse text symbol".to_string(),
                                ));
                            }
                        }
                        true => {
                            // reads the text data
                            text = String::from_utf8(bytes_text.to_vec())?;
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
        Ok(TextObject {
            geometry: text_geo,
            text,
            h_align: h_align.unwrap_or(HorizontalAlign::HCenter),
            v_align: v_align.unwrap_or(VerticalAlign::VCenter),
            rotation,
        })
    }
}
