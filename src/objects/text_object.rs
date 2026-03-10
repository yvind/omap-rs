use std::{cell::RefCell, collections::HashMap, rc::Weak, str::FromStr};

use crate::{
    Error, Result,
    symbols::TextSymbol,
    utils::{from_file_coords, from_file_value, to_file_coords, to_file_value, try_get_attr},
};
use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

/// The geometry of a text object, which is either a single anchor or a wrap box.
#[derive(Debug, Clone)]
pub enum TextGeometry {
    /// A single anchor point.
    SingleAnchor(Coord),
    /// A rectangular bounding box for wrapped text.
    WrapBox(WrapBox),
}

impl TextGeometry {
    /// Get a shared reference to the anchor coordinate.
    pub fn get_anchor_coord(&self) -> &Coord {
        match self {
            TextGeometry::SingleAnchor(coord) => coord,
            TextGeometry::WrapBox(wrap_box) => &wrap_box.anchor,
        }
    }

    /// Get a mutable reference to the anchor coordinate.
    pub fn get_anchor_coord_mut(&mut self) -> &mut Coord {
        match self {
            TextGeometry::SingleAnchor(coord) => coord,
            TextGeometry::WrapBox(wrap_box) => &mut wrap_box.anchor,
        }
    }
}

/// A rectangular bounding box for wrapped text.
#[derive(Debug, Clone, Default)]
pub struct WrapBox {
    /// The anchor (origin) coordinate of the box.
    pub anchor: Coord,
    /// Width of the text box in mm
    pub width: f64,
    /// Height of the text box in mm
    pub height: f64,
}

/// Horizontal text alignment.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    /// Align to the left.
    Left = 0,
    /// Centre horizontally.
    #[default]
    HCenter = 1,
    /// Align to the right.
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

/// Vertical text alignment.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    /// Align to the text baseline.
    Baseline = 0,
    /// Align to the top.
    Top = 1,
    /// Centre vertically.
    #[default]
    VCenter = 2,
    /// Align to the bottom.
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

/// A text object placed on the map.
#[derive(Debug, Clone)]
pub struct TextObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    /// Weak reference to the text symbol used to render this object.
    pub symbol: Weak<RefCell<TextSymbol>>,
    /// The text geometry (anchor or wrap box).
    pub geometry: TextGeometry,
    /// The text content.
    pub text: String,
    /// Horizontal alignment.
    pub h_align: HorizontalAlign,
    /// Vertical alignment.
    pub v_align: VerticalAlign,
    /// Rotation of the text in radians.
    pub rotation: f64,
}

impl TextObject {
    /// Write just the inner content (coords + size + text) — called from MapObject::write
    pub(super) fn write_content<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        match &self.geometry {
            TextGeometry::SingleAnchor(coord) => {
                let fc = to_file_coords(*coord)?;
                let bs = BytesStart::new("coords").with_attributes([("count", "1")]);
                writer.write_event(Event::Start(bs))?;
                writer.write_event(Event::Text(BytesText::new(&format!("{} {};", fc.x, fc.y))))?;
                writer.write_event(Event::End(BytesEnd::new("coords")))?;
            }
            TextGeometry::WrapBox(wb) => {
                let fc = to_file_coords(wb.anchor)?;
                let width = to_file_value(wb.width)?;
                let height = to_file_value(wb.height)?;
                let bs = BytesStart::new("coords").with_attributes([("count", "2")]);
                writer.write_event(Event::Start(bs))?;
                writer.write_event(Event::Text(BytesText::new(&format!(
                    "{} {};{} {};",
                    fc.x, fc.y, width, height
                ))))?;
                writer.write_event(Event::End(BytesEnd::new("coords")))?;

                // Write <size> element for the wrap box
                writer.write_event(Event::Empty(BytesStart::new("size").with_attributes([
                    ("width", width.to_string().as_str()),
                    ("height", height.to_string().as_str()),
                ])))?;
            }
        }

        // Write text content
        writer.write_event(Event::Start(BytesStart::new("text")))?;
        writer.write_event(Event::Text(BytesText::new(&self.text)))?;
        writer.write_event(Event::End(BytesEnd::new("text")))?;

        Ok(())
    }

    /// Parse a text object. The reader should be positioned right after
    /// the `<coords>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        coords_element: &BytesStart<'_>,
        h_align: HorizontalAlign,
        v_align: VerticalAlign,
        rotation: f64,
    ) -> Result<TextObject> {
        let num_coords: usize = try_get_attr(coords_element, "count").unwrap_or(0);
        if num_coords == 0 {
            return Err(Error::ParseOmapFileError(
                "Text object has 0 coords".to_string(),
            ));
        }

        let mut text_geo = if num_coords == 1 {
            TextGeometry::SingleAnchor(Coord::default())
        } else {
            TextGeometry::WrapBox(WrapBox::default())
        };
        let mut is_coords_read = false;
        let mut text = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if bytes_start.local_name().as_ref() == b"size" {
                        // Override box size from <size> element (takes precedence)
                        let w: i32 = try_get_attr(&bytes_start, "width").unwrap_or(0);
                        let h: i32 = try_get_attr(&bytes_start, "height").unwrap_or(0);
                        if let TextGeometry::WrapBox(ref mut wb) = text_geo {
                            wb.width = from_file_value(w);
                            wb.height = from_file_value(h);
                        }
                    }
                }
                Event::Empty(bytes_start) => {
                    if bytes_start.local_name().as_ref() == b"size" {
                        let w: i32 = try_get_attr(&bytes_start, "width").unwrap_or(0);
                        let h: i32 = try_get_attr(&bytes_start, "height").unwrap_or(0);
                        if let TextGeometry::WrapBox(ref mut wb) = text_geo {
                            wb.width = from_file_value(w);
                            wb.height = from_file_value(h);
                        }
                    }
                }
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

                            if let Some((coords_str, opt_wh)) = raw_xml.split_once(';') {
                                let mut split = coords_str.split_whitespace();

                                let x: i32 = split
                                    .next()
                                    .ok_or(Error::InvalidCoordinate("No x value".to_string()))?
                                    .parse()?;
                                let y: i32 = split
                                    .next()
                                    .ok_or(Error::InvalidCoordinate("No y value".to_string()))?
                                    .parse()?;

                                let coord = from_file_coords(Coord { x, y });

                                // Parse second coord (box size) if present
                                let box_size = if !opt_wh.is_empty() {
                                    // opt_wh might be "w h;" or "w h;rest..."
                                    let wh_str = opt_wh.split(';').next().unwrap_or("");
                                    if !wh_str.is_empty() {
                                        let mut wh_split = wh_str.split_whitespace();
                                        let w: i32 = wh_split
                                            .next()
                                            .and_then(|s| s.parse().ok())
                                            .unwrap_or(0);
                                        let h: i32 = wh_split
                                            .next()
                                            .and_then(|s| s.parse().ok())
                                            .unwrap_or(0);
                                        Some((from_file_value(w), from_file_value(h)))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                match &mut text_geo {
                                    TextGeometry::SingleAnchor(point) => *point = coord,
                                    TextGeometry::WrapBox(wrap_box) => {
                                        wrap_box.anchor = coord;
                                        if let Some((w, h)) = box_size {
                                            wrap_box.width = w;
                                            wrap_box.height = h;
                                        }
                                    }
                                };
                            } else {
                                return Err(Error::ParseOmapFileError(
                                    "Could not parse text object coords".to_string(),
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
                        "Unexpected EOF in TextObject parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }
        Ok(TextObject {
            tags: HashMap::new(),
            symbol: Weak::new(),
            geometry: text_geo,
            text,
            h_align,
            v_align,
            rotation,
        })
    }
}
