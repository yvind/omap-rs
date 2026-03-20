use std::{cell::RefCell, collections::HashMap, rc::Weak, str::FromStr};

use crate::{
    Error, NonNegativeF64, Result, notes,
    symbols::{Symbol, SymbolSet, TextSymbol},
    utils::{from_file_coords, to_file_coords, try_get_attr_raw},
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
    pub width: NonNegativeF64,
    /// Height of the text box in mm
    pub height: NonNegativeF64,
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
    /// Create a new text object with the given symbol, geometry, and text content.
    pub fn new(symbol: Weak<RefCell<TextSymbol>>, geometry: TextGeometry, text: String) -> Self {
        TextObject {
            tags: HashMap::new(),
            symbol,
            geometry,
            text,
            h_align: HorizontalAlign::default(),
            v_align: VerticalAlign::default(),
            rotation: 0.0,
        }
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let mut is_rotatable = false;
        let index = if let Some(sym) = self.symbol.upgrade() {
            is_rotatable = sym.try_borrow().map(|t| t.is_rotatable).unwrap_or(false);
            symbol_set
                .iter()
                .position(|s| {
                    if let Symbol::Text(s) = s {
                        s.as_ptr() == sym.as_ptr()
                    } else {
                        false
                    }
                })
                .map(|p| p as i32)
                .unwrap_or(-1)
        } else {
            -1
        };

        let mut bs = BytesStart::new("object").with_attributes([
            ("type", "4"),
            ("symbol", index.to_string().as_str()),
            ("h_align", (self.h_align as u8).to_string().as_str()),
            ("v_align", (self.v_align as u8).to_string().as_str()),
        ]);

        if self.rotation.abs() > f64::EPSILON && is_rotatable {
            // Map the rotation onto [-PI, PI]
            // first shift the target to either (-TAU, 0] for negative or [0, TAU) for positive
            // Take the modulus with TAU (negatives return negative values) and shift target back to [-PI, PI]
            let rot = (self.rotation + self.rotation.signum() * std::f64::consts::PI)
                % std::f64::consts::TAU
                - self.rotation.signum() * std::f64::consts::PI;
            bs.push_attribute(("rotation", rot.to_string().as_str()));
        }
        writer.write_event(Event::Start(bs))?;
        if !self.tags.is_empty() {
            super::write_tags(writer, &self.tags)?;
        }

        match &self.geometry {
            TextGeometry::SingleAnchor(coord) => {
                writer.write_event(Event::Start(
                    BytesStart::new("coords").with_attributes([("count", "1")]),
                ))?;
                let fc = to_file_coords(*coord)?;
                writer.write_event(Event::Text(BytesText::new(&format!("{} {};", fc.x, fc.y))))?;
                writer.write_event(Event::End(BytesEnd::new("coords")))?;
            }
            TextGeometry::WrapBox(wb) => {
                writer.write_event(Event::Start(
                    BytesStart::new("coords").with_attributes([("count", "2")]),
                ))?;
                let fc = to_file_coords(wb.anchor)?;
                let width = wb.width.to_file_value()?;
                let height = wb.height.to_file_value()?;
                writer.write_event(Event::Text(BytesText::new(&format!(
                    "{} {};{} {};",
                    fc.x, fc.y, width, height
                ))))?;
                writer.write_event(Event::End(BytesEnd::new("coords")))?;
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
        writer.write_event(Event::End(BytesEnd::new("coords")))?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Parse a text object. The reader should be positioned right after
    /// the `<object>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: Weak<RefCell<TextSymbol>>,
        h_align: HorizontalAlign,
        v_align: VerticalAlign,
        rotation: f64,
    ) -> Result<TextObject> {
        let mut text_geo = TextGeometry::SingleAnchor(Coord::default());
        let mut tags = HashMap::new();
        let mut text = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    match bytes_start.local_name().as_ref() {
                        b"tags" => tags = super::parse_tags(reader)?,
                        b"size" => {
                            // Override box size from <size> element (takes precedence)
                            let w = try_get_attr_raw(&bytes_start, "width").unwrap_or(0);
                            let h = try_get_attr_raw(&bytes_start, "height").unwrap_or(0);
                            if let TextGeometry::WrapBox(wb) = &mut text_geo {
                                wb.width = NonNegativeF64::from_file_value(w);
                                wb.height = NonNegativeF64::from_file_value(h);
                            }
                        }
                        b"coords" => match try_get_attr_raw::<u8>(&bytes_start, "count") {
                            Some(1) => text_geo = TextGeometry::SingleAnchor(Coord::default()),
                            Some(2) => text_geo = TextGeometry::WrapBox(WrapBox::default()),
                            _ => return Err(Error::ObjectError),
                        },
                        b"text" => text = notes::parse(reader)?,
                        _ => (),
                    }
                }
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    // parse the text location
                    let raw_xml = str::from_utf8(bytes_text.as_ref())?;

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
                            if let Some(wh_str) = opt_wh.split(';').next() {
                                let mut wh_split = wh_str.split_whitespace();
                                let w = wh_split.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                                let h = wh_split.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                                Some((
                                    NonNegativeF64::from_file_value(w),
                                    NonNegativeF64::from_file_value(h),
                                ))
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
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in TextObject parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }
        Ok(TextObject {
            tags,
            symbol,
            geometry: text_geo,
            text,
            h_align,
            v_align,
            rotation,
        })
    }
}
