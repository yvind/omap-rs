use crate::editor::{Result, Transform};
use geo_types::Point;
use quick_xml::Reader;

#[derive(Debug, Clone)]
pub enum TextGeomtry {
    SingleAnchor(Point),
    WrapBox(WrapBox),
}

#[derive(Debug, Clone)]
pub struct WrapBox {
    anchor: Point,
    height: f64,
    width: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    AlignLeft = 0,
    AlignHCenter = 1,
    AlignRight = 2,
}

impl HorizontalAlign {
    pub(super) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"0" => Some(HorizontalAlign::AlignLeft),
            b"1" => Some(HorizontalAlign::AlignHCenter),
            b"2" => Some(HorizontalAlign::AlignRight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    AlignBaseline = 0,
    AlignTop = 1,
    AlignVCenter = 2,
    AlignBottom = 3,
}

impl VerticalAlign {
    pub(super) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"0" => Some(VerticalAlign::AlignBaseline),
            b"1" => Some(VerticalAlign::AlignTop),
            b"2" => Some(VerticalAlign::AlignVCenter),
            b"3" => Some(VerticalAlign::AlignBottom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextObject {
    pub geometry: TextGeomtry,
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

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        let coords_tag = match self.geometry {
            TextGeomtry::SingleAnchor(p) => {
                let map_coords = transform.to_map_coords(p.0);
                format!(
                    "<coords count=\"1\">{} {};</coords>",
                    map_coords.0, map_coords.1
                )
            }
            TextGeomtry::WrapBox(wp) => {
                let map_coords = transform.to_map_coords(wp.anchor.0);
                let width = transform.to_map_dist(wp.width);
                let height = transform.to_map_dist(wp.height);

                format!(
                    "<coords count=\"2\">{} {};{} {};</coords><size width=\"{}\" height=\"{}\"/>",
                    map_coords.0, map_coords.1, width, height, width, height
                )
            }
        };

        writer.write_all(format!("{}<text>{}</text>", coords_tag, self.text).as_bytes())?;
        Ok(())
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        h_align: Option<HorizontalAlign>,
        v_align: Option<VerticalAlign>,
        rotation: f64,
    ) -> Result<(Self, String)> {
    }
}
