use std::{cell::RefCell, collections::HashMap, rc::Weak, str::FromStr};

use crate::{Error, Result, symbols::TextSymbol};
use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesStart, Event},
};

#[derive(Debug, Clone)]
pub enum TextGeometry {
    SingleAnchor(Coord),
    WrapBox(WrapBox),
}

impl TextGeometry {
    pub fn get_anchor_coord(&self) -> &Coord {
        match self {
            TextGeometry::SingleAnchor(coord) => coord,
            TextGeometry::WrapBox(wrap_box) => &wrap_box.anchor,
        }
    }

    pub fn get_anchor_coord_mut(&mut self) -> &mut Coord {
        match self {
            TextGeometry::SingleAnchor(coord) => coord,
            TextGeometry::WrapBox(wrap_box) => &mut wrap_box.anchor,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WrapBox {
    pub anchor: Coord,
    pub width: f64,
    pub height: f64,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    Left = 0,
    #[default]
    HCenter = 1,
    Right = 2,
}

impl FromStr for HorizontalAlign {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "0" => Ok(HorizontalAlign::Left),
            "1" => Ok(HorizontalAlign::HCenter),
            "2" => Ok(HorizontalAlign::Right),
            _ => Err(Error::ObjectError),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    Baseline = 0,
    Top = 1,
    #[default]
    VCenter = 2,
    Bottom = 3,
}

impl FromStr for VerticalAlign {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "0" => Ok(VerticalAlign::Baseline),
            "1" => Ok(VerticalAlign::Top),
            "2" => Ok(VerticalAlign::VCenter),
            "3" => Ok(VerticalAlign::Bottom),
            _ => Err(Error::ObjectError),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    pub symbol: Weak<RefCell<TextSymbol>>,
    pub geometry: TextGeometry,
    pub text: String,
    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
    pub rotation: f64,
}

impl TextObject {
    pub(crate) fn get_special_keys(&self) -> Option<String> {
        // also check if the symbol allows rotation
        if self.rotation.is_normal() && self.rotation.abs() > 0.01 {
            Some(format!(
                "rotation=\"{:.3}\" h_align=\"{}\" v_align=\"{}\"",
                self.rotation, self.h_align as u8, self.v_align as u8
            ))
        } else {
            Some(format!(
                "h_align=\"{}\" v_align=\"{}\"",
                self.h_align as u8, self.v_align as u8
            ))
        }
    }

    pub(crate) fn write<W: std::io::Write>(self, _writer: &mut Writer<W>) -> Result<()> {
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
    ) -> Result<(Self, String)> {
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
            TextGeometry::SingleAnchor(Coord::default())
        } else {
            TextGeometry::WrapBox(WrapBox::default())
        };
        let mut is_coords_read = false;
        let mut raw_xml = String::new();
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

                            raw_xml = String::from_utf8(bytes_text.to_vec())?;

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
                                    TextGeometry::SingleAnchor(point) => *point = coord,
                                    TextGeometry::WrapBox(wrap_box) => {
                                        wrap_box.anchor = coord;
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
                            text.push_str(&bytes_text.xml_content()?);
                        }
                    }
                }
                Event::GeneralRef(bytes_ref) => {
                    text.push_str(&quick_xml::escape::unescape(&format!(
                        "&{};",
                        &bytes_ref.xml_content()?
                    ))?);
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
            TextObject {
                geometry: text_geo,
                text,
                h_align: h_align.unwrap_or(HorizontalAlign::HCenter),
                v_align: v_align.unwrap_or(VerticalAlign::VCenter),
                rotation,
            },
            raw_xml,
        ))
    }
}
