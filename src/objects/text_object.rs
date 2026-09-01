use std::{collections::HashMap, str::FromStr};

use crate::{
    CoordinateComponent, Error, NonNegativeF64, OmapSection, Result, notes,
    symbols::{SymbolSet, TextSymbolId},
    utils::{
        from_file_coords, to_file_coords, transform_position, try_get_attr_raw,
        try_transform_position,
    },
};
use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

/// The geometry of a text object, which is either a single anchor or a wrap box.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextGeometry {
    /// A single anchor point.
    SingleAnchor(Coord),
    /// A rectangular bounding box for wrapped text.
    WrapBox(WrapBox),
}

impl TextGeometry {
    /// Get a shared reference to the anchor coordinate.
    pub fn anchor_coord(&self) -> &Coord {
        match self {
            Self::SingleAnchor(coord) => coord,
            Self::WrapBox(wrap_box) => &wrap_box.anchor,
        }
    }

    /// Get a mutable reference to the anchor coordinate.
    pub fn anchor_coord_mut(&mut self) -> &mut Coord {
        match self {
            Self::SingleAnchor(coord) => coord,
            Self::WrapBox(wrap_box) => &mut wrap_box.anchor,
        }
    }
}

/// A rectangular bounding box for wrapped text.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            "0" => Ok(Self::Left),
            "1" => Ok(Self::HCenter),
            "2" => Ok(Self::Right),
            _ => Err(Error::ObjectError),
        }
    }
}

/// Vertical text alignment.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            "0" => Ok(Self::Baseline),
            "1" => Ok(Self::Top),
            "2" => Ok(Self::VCenter),
            "3" => Ok(Self::Bottom),
            _ => Err(Error::ObjectError),
        }
    }
}

/// A text object placed on the map.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextObject {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    #[expect(
        clippy::box_collection,
        reason = "the map header is 48 bytes inline and most objects carry no tags"
    )]
    tags: Option<Box<HashMap<String, String>>>,
    /// Weak reference to the text symbol used to render this object.
    pub symbol: Option<TextSymbolId>,
    geometry: TextGeometry,
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
    pub fn new(symbol: Option<TextSymbolId>, geometry: TextGeometry, text: String) -> Self {
        Self {
            tags: None,
            symbol,
            geometry,
            text,
            h_align: HorizontalAlign::default(),
            v_align: VerticalAlign::default(),
            rotation: 0.0,
        }
    }

    /// The tags associated with the object.
    pub fn tags(&self) -> &HashMap<String, String> {
        match &self.tags {
            Some(tags) => tags,
            None => super::empty_tags(),
        }
    }

    /// Mutably access the tags, allocating the map on first use.
    pub fn tags_mut(&mut self) -> &mut HashMap<String, String> {
        self.tags.get_or_insert_with(Box::default)
    }

    /// Get a shared reference to the text geometry.
    pub fn geometry(&self) -> &TextGeometry {
        &self.geometry
    }

    /// Get a mutable reference to the text geometry.
    pub fn geometry_mut(&mut self) -> &mut TextGeometry {
        &mut self.geometry
    }

    /// Transform the text anchor and rotation.
    pub fn transform<F>(&mut self, transform: F)
    where
        F: Fn(Coord) -> Coord,
    {
        let anchor = match &self.geometry {
            TextGeometry::SingleAnchor(coord) => *coord,
            TextGeometry::WrapBox(wrap_box) => wrap_box.anchor,
        };
        let (anchor, rotation, _) = transform_position(anchor, transform);

        match &mut self.geometry {
            TextGeometry::SingleAnchor(coord) => *coord = anchor,
            TextGeometry::WrapBox(wrap_box) => wrap_box.anchor = anchor,
        }
        self.rotation += rotation;
    }

    /// Try to transform the text anchor and rotation.
    ///
    /// # Errors
    ///
    /// Returns any error produced by `transform`. The object is unchanged on
    /// failure.
    pub fn try_transform<E, F>(&mut self, transform: F) -> std::result::Result<(), E>
    where
        F: Fn(Coord) -> std::result::Result<Coord, E>,
    {
        let anchor = match &self.geometry {
            TextGeometry::SingleAnchor(coord) => *coord,
            TextGeometry::WrapBox(wrap_box) => wrap_box.anchor,
        };
        let (anchor, rotation, _) = try_transform_position(anchor, transform)?;

        match &mut self.geometry {
            TextGeometry::SingleAnchor(coord) => *coord = anchor,
            TextGeometry::WrapBox(wrap_box) => wrap_box.anchor = anchor,
        }
        self.rotation += rotation;
        Ok(())
    }

    /// Consume this object and return its geometry.
    pub fn into_geometry(self) -> TextGeometry {
        self.geometry
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let is_rotatable = self
            .symbol
            .and_then(|id| symbol_set.text_symbol(id))
            .is_some_and(|symbol| symbol.is_rotatable);
        let index = symbol_set.file_index(self.symbol);

        let mut bs = BytesStart::new("object").with_attributes([
            ("type", "4"),
            ("symbol", index.to_string().as_str()),
            ("h_align", (self.h_align as u8).to_string().as_str()),
            ("v_align", (self.v_align as u8).to_string().as_str()),
        ]);

        if self.rotation.abs() > f64::EPSILON && is_rotatable {
            // Map the rotation onto [-PI, PI].
            let rot = (self.rotation + self.rotation.signum() * std::f64::consts::PI)
                % std::f64::consts::TAU
                - self.rotation.signum() * std::f64::consts::PI;
            bs.push_attribute(("rotation", rot.to_string().as_str()));
        }
        writer.write_event(Event::Start(bs))?;
        if !self.tags().is_empty() {
            super::write_tags(writer, self.tags())?;
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
        writer.write_event(Event::Start(BytesStart::new("text")))?;
        writer.write_event(Event::Text(BytesText::new(&self.text)))?;
        writer.write_event(Event::End(BytesEnd::new("text")))?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Parse a text object. The reader should be positioned right after
    /// the `<object>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: Option<TextSymbolId>,
        h_align: HorizontalAlign,
        v_align: VerticalAlign,
        rotation: f64,
    ) -> Result<Self> {
        let mut text_geo = TextGeometry::SingleAnchor(Coord::default());
        let mut tags = None;
        let mut text = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    match bytes_start.local_name().as_ref() {
                        "tags" => tags = super::parse_tags(reader)?,
                        "size" => {
                            // Override box size from <size> element (takes precedence)
                            let w = try_get_attr_raw(&bytes_start, "width")?.unwrap_or(0);
                            let h = try_get_attr_raw(&bytes_start, "height")?.unwrap_or(0);
                            if let TextGeometry::WrapBox(wb) = &mut text_geo {
                                wb.width = NonNegativeF64::from_file_value(w);
                                wb.height = NonNegativeF64::from_file_value(h);
                            }
                        }
                        "coords" => match try_get_attr_raw::<u8>(&bytes_start, "count")? {
                            Some(1) => text_geo = TextGeometry::SingleAnchor(Coord::default()),
                            Some(2) => text_geo = TextGeometry::WrapBox(WrapBox::default()),
                            _ => return Err(Error::ObjectError),
                        },
                        "text" => text = notes::parse(reader)?,
                        _ => (),
                    }
                }
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), "object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    let raw_xml = bytes_text.as_ref();

                    if let Some((coords_str, opt_wh)) = raw_xml.split_once(';') {
                        let mut split = coords_str.split_whitespace();

                        let x: i32 = split
                            .next()
                            .ok_or(Error::MissingCoordinateComponent(CoordinateComponent::X))?
                            .parse()?;
                        let y: i32 = split
                            .next()
                            .ok_or(Error::MissingCoordinateComponent(CoordinateComponent::Y))?
                            .parse()?;

                        let coord = from_file_coords(Coord { x, y });

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
                        }
                    } else {
                        return Err(Error::MissingTextObjectCoordinates);
                    }
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::TextObject));
                }
                _ => (),
            }
        }
        Ok(Self {
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
