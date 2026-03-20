use std::str::FromStr;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{PointSymbol, SymbolCommon};
use crate::{
    Code, Error, NonNegativeF64, Result,
    colors::{ColorSet, SymbolColor},
    utils::{parse_attr, try_get_attr_raw},
};

/// A fill pattern applied to an area.
#[derive(Debug, Clone)]
pub enum FillPattern {
    /// A pattern of parallel lines.
    LinePattern {
        angle: f64,
        line_spacing: NonNegativeF64,
        line_offset: NonNegativeF64,
        line_color: SymbolColor,
        line_width: NonNegativeF64,
        rotatable: bool, // stored as flag 16 with the clip options
    },
    /// A pattern of regularly spaced points.
    PointPattern {
        clip_options: ClippingOption,
        angle: f64,
        line_spacing: NonNegativeF64,
        line_offset: NonNegativeF64,
        offset_along_line: NonNegativeF64,
        point_distance: NonNegativeF64,
        point: PointSymbol,
        rotatable: bool, // stored as flag 16 with the clip options
    },
}

/// Clipping option for point patterns at area boundaries.
#[derive(Debug, Clone, Copy, Default)]
pub enum ClippingOption {
    /// Clip elements at the boundary.
    #[default]
    ClipElementsAtBoundary = 0,
    /// No clipping if the element is completely inside.
    NoClippingIfCompletelyInside = 1,
    /// No clipping if the element centre is inside.
    NoClippingIfCenterInside = 2,
    /// No clipping if the element is at least partially inside.
    NoClippingIfPartiallyInside = 3,
}

impl FromStr for ClippingOption {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "0" => Ok(ClippingOption::ClipElementsAtBoundary),
            "1" => Ok(ClippingOption::NoClippingIfCompletelyInside),
            "2" => Ok(ClippingOption::NoClippingIfCenterInside),
            "3" => Ok(ClippingOption::NoClippingIfPartiallyInside),
            _ => Err(Error::SymbolError(format!("Unknown ClippingOption {s}"))),
        }
    }
}

impl FillPattern {
    fn parse<R: std::io::BufRead>(
        element: &BytesStart<'_>,
        reader: &mut Reader<R>,
        color_set: &ColorSet,
    ) -> Result<FillPattern> {
        let pattern_type = try_get_attr_raw(element, "type").unwrap_or(0);
        let angle = try_get_attr_raw(element, "angle").unwrap_or(0.0);
        let clip_options = try_get_attr_raw(element, "no_clipping").unwrap_or_default();
        let rotatable = try_get_attr_raw(element, "rotatable").unwrap_or(false);
        let line_spacing =
            NonNegativeF64::from_file_value(try_get_attr_raw(element, "line_spacing").unwrap_or(0));
        let line_offset =
            NonNegativeF64::from_file_value(try_get_attr_raw(element, "line_offset").unwrap_or(0));
        let offset_along_line = NonNegativeF64::from_file_value(
            try_get_attr_raw(element, "offset_along_line").unwrap_or(0),
        );

        match pattern_type {
            1 => {
                // LinePattern
                let ci = try_get_attr_raw(element, "color").unwrap_or(-1);
                let line_color = SymbolColor::from_index(ci, color_set);
                let line_width = NonNegativeF64::from_file_value(
                    try_get_attr_raw(element, "line_width").unwrap_or(0),
                );
                // Skip to end of pattern element
                let mut buf = Vec::new();
                loop {
                    match reader.read_event_into(&mut buf)? {
                        Event::End(e) => {
                            if e.local_name().as_ref() == b"pattern" {
                                break;
                            }
                        }
                        Event::Eof => {
                            return Err(Error::ParseOmapFileError(
                                "Unexpected EOF in FillPattern parsing".to_string(),
                            ));
                        }
                        _ => {}
                    }
                }
                Ok(FillPattern::LinePattern {
                    angle,
                    line_spacing,
                    line_offset,
                    line_color,
                    line_width,
                    rotatable,
                })
            }
            2 => {
                // PointPattern
                let point_distance = NonNegativeF64::from_file_value(
                    try_get_attr_raw(element, "point_distance").unwrap_or(0),
                );
                // Parse nested point symbol
                let mut point = None;
                let mut buf = Vec::new();
                loop {
                    match reader.read_event_into(&mut buf)? {
                        Event::Start(e) => {
                            if e.local_name().as_ref() == b"symbol" {
                                let mut sub_common = SymbolCommon::default();
                                for attr in e.attributes().filter_map(std::result::Result::ok) {
                                    match attr.key.local_name().as_ref() {
                                        b"name" => {
                                            sub_common.name = parse_attr(attr, e.decoder())
                                                .unwrap_or(sub_common.name);
                                        }
                                        b"code" => {
                                            sub_common.code =
                                                crate::utils::parse_attr_raw(attr.value)
                                                    .unwrap_or_default();
                                        }
                                        _ => {}
                                    }
                                }
                                point = Some(PointSymbol::parse(reader, color_set, sub_common)?);
                            }
                        }
                        Event::End(e) => {
                            if e.local_name().as_ref() == b"pattern" {
                                break;
                            }
                        }
                        Event::Eof => {
                            return Err(Error::ParseOmapFileError(
                                "Unexpected EOF in FillPattern point parsing".to_string(),
                            ));
                        }
                        _ => {}
                    }
                }
                let point = point.ok_or_else(|| {
                    Error::ParseOmapFileError("Missing point symbol in PointPattern".to_string())
                })?;
                Ok(FillPattern::PointPattern {
                    clip_options,
                    angle,
                    line_spacing,
                    line_offset,
                    offset_along_line,
                    point_distance,
                    point,
                    rotatable,
                })
            }
            _ => Err(Error::ParseOmapFileError(format!(
                "Unknown fill pattern type {pattern_type}"
            ))),
        }
    }

    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>, color_set: &ColorSet) -> Result<()> {
        match self {
            FillPattern::LinePattern {
                angle,
                line_spacing,
                line_offset,
                line_color,
                line_width,
                rotatable,
            } => {
                let mut bs = BytesStart::new("pattern")
                    .with_attributes([("type", "1"), ("angle", angle.to_string().as_str())]);
                if *rotatable {
                    bs.push_attribute(("rotatable", "true"));
                }
                bs.push_attribute((
                    "line_spacing",
                    line_spacing.to_file_value()?.to_string().as_str(),
                ));
                bs.push_attribute((
                    "line_offset",
                    line_offset.to_file_value()?.to_string().as_str(),
                ));
                bs.push_attribute(("offset_along_line", "0"));
                bs.push_attribute((
                    "color",
                    line_color.get_priority(color_set).to_string().as_str(),
                ));
                bs.push_attribute((
                    "line_width",
                    line_width.to_file_value()?.to_string().as_str(),
                ));
                writer.write_event(Event::Empty(bs))?;
            }
            FillPattern::PointPattern {
                clip_options,
                angle,
                line_spacing,
                line_offset,
                offset_along_line,
                point_distance,
                point,
                rotatable,
            } => {
                let mut bs = BytesStart::new("pattern")
                    .with_attributes([("type", "2"), ("angle", angle.to_string().as_str())]);
                if *clip_options as u8 > 0 {
                    bs.push_attribute(("no_clipping", (*clip_options as u8).to_string().as_str()));
                }
                if *rotatable {
                    bs.push_attribute(("rotatable", "true"));
                }
                bs.push_attribute((
                    "line_spacing",
                    line_spacing.to_file_value()?.to_string().as_str(),
                ));
                bs.push_attribute((
                    "line_offset",
                    line_offset.to_file_value()?.to_string().as_str(),
                ));
                bs.push_attribute((
                    "offset_along_line",
                    offset_along_line.to_file_value()?.to_string().as_str(),
                ));
                bs.push_attribute((
                    "point_distance",
                    point_distance.to_file_value()?.to_string().as_str(),
                ));
                writer.write_event(Event::Start(bs))?;
                point.write(writer, color_set, None)?;
                writer.write_event(Event::End(BytesEnd::new("pattern")))?;
            }
        }
        Ok(())
    }
}

/// An area symbol definition.
#[derive(Debug, Clone)]
pub struct AreaSymbol {
    /// Common symbol properties.
    pub common: SymbolCommon,

    /// Whether the fill pattern is rotatable.
    pub is_rotatable: bool,

    /// The area fill colour.
    pub color: SymbolColor,
    /// Fill patterns applied to the area.
    pub patterns: Vec<FillPattern>,
    /// Minimum area in mm² for the symbol to be drawn.
    pub minimum_area: NonNegativeF64,
}

impl AreaSymbol {
    /// Get the display name of this area symbol.
    pub fn get_name(&self) -> &str {
        &self.common.name
    }

    /// Create a new empty area symbol with the given code and name.
    pub fn new(code: Code, name: String) -> AreaSymbol {
        let common = SymbolCommon {
            code,
            name,
            ..Default::default()
        };
        AreaSymbol {
            common,
            is_rotatable: true,
            color: SymbolColor::NoColor,
            patterns: Vec::new(),
            minimum_area: NonNegativeF64::clamped_from(0.),
        }
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<AreaSymbol> {
        let mut common = attributes;
        let mut color = SymbolColor::NoColor;
        let mut minimum_area = NonNegativeF64::default();
        let mut is_rotatable = false;
        let mut patterns = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"description" => {
                        if let Event::Text(text) = reader.read_event_into(&mut buf)? {
                            common.description = String::from_utf8(text.to_vec())?;
                        }
                    }
                    b"area_symbol" => {
                        let ci = try_get_attr_raw(&e, "inner_color").unwrap_or(-1);
                        color = SymbolColor::from_index(ci, color_set);
                        minimum_area = NonNegativeF64::from_file_value(
                            try_get_attr_raw(&e, "min_area").unwrap_or(0),
                        );
                        is_rotatable = try_get_attr_raw(&e, "rotatable").unwrap_or(false);
                    }
                    b"pattern" => {
                        patterns.push(FillPattern::parse(&e, reader, color_set)?);
                    }
                    _ => {}
                },
                Event::Empty(e) => {
                    if e.local_name().as_ref() == b"icon"
                        && let Some(src) = try_get_attr_raw(&e, "src")
                    {
                        common.custom_icon = Some(src);
                    }
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == b"symbol" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in AreaSymbol parsing".to_string(),
                    ));
                }
                _ => {}
            }
        }

        Ok(AreaSymbol {
            common,
            is_rotatable,
            color,
            patterns,
            minimum_area,
        })
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
        index: Option<usize>,
    ) -> Result<()> {
        let mut bs = BytesStart::new("symbol").with_attributes([
            ("type", "4"),
            ("code", self.common.code.to_string().as_str()),
            (
                "name",
                quick_xml::escape::escape(self.common.name.as_str()).as_ref(),
            ),
        ]);
        if let Some(id) = index {
            bs.push_attribute(("id", id.to_string().as_str()));
        }
        if self.common.is_hidden {
            bs.push_attribute(("is_hidden", "true"));
        }
        if self.common.is_helper_symbol {
            bs.push_attribute(("is_helper_symbol", "true"));
        }
        if self.common.is_protected {
            bs.push_attribute(("is_protected", "true"));
        }
        writer.write_event(Event::Start(bs))?;

        if !self.common.description.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("description")))?;
            writer.write_event(Event::Text(BytesText::new(&self.common.description)))?;
            writer.write_event(Event::End(BytesEnd::new("description")))?;
        }

        let mut bs = BytesStart::new("area_symbol");
        bs.push_attribute((
            "inner_color",
            self.color.get_priority(color_set).to_string().as_str(),
        ));
        bs.push_attribute((
            "min_area",
            self.minimum_area.to_file_value()?.to_string().as_str(),
        ));
        if self.is_rotatable {
            bs.push_attribute(("rotatable", "true"));
        }
        bs.push_attribute(("patterns", self.patterns.len().to_string().as_str()));
        writer.write_event(Event::Start(bs))?;

        for pattern in &self.patterns {
            pattern.write(writer, color_set)?;
        }

        writer.write_event(Event::End(BytesEnd::new("area_symbol")))?;

        if let Some(icon) = &self.common.custom_icon {
            writer.write_event(Event::Empty(
                BytesStart::new("icon").with_attributes([("src", icon.as_str())]),
            ))?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbol")))?;
        Ok(())
    }
}
