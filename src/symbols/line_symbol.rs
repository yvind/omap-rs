use std::str::FromStr;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{PointSymbol, SymbolCommon};
use crate::{
    Error, NonNegativeF64, Result,
    colors::{ColorSet, SymbolColor},
    utils::try_get_attr,
};

#[derive(Debug, Clone)]
pub struct LineSymbol {
    pub common: SymbolCommon,

    pub border: Option<BorderStyle>,

    pub start_symbol: Option<PointSymbol>,
    pub mid_symbol: Option<MidSymbol>,
    pub end_symbol: Option<PointSymbol>,
    pub dash_symbol: Option<DashSymbol>,

    pub color: SymbolColor,
    pub line_width: NonNegativeF64,
    pub minimum_length: NonNegativeF64,
    pub start_offset: NonNegativeF64,
    pub end_offset: NonNegativeF64,

    pub dash_style: DashStyle,
    pub cap_style: CapStyle,
    pub join_style: JoinStyle,
}

#[derive(Debug, Clone)]
pub struct DashSymbol {
    pub suppress_dash_symbol_at_ends: bool,
    pub scale_dash_symbol: bool,
    pub dash_symbol: PointSymbol,
}

#[derive(Debug, Clone)]
pub struct MidSymbol {
    /// Number of mid symbols per
    pub mid_symbols_per_spot: u16,
    /// Distance in mm
    pub mid_symbol_distance: NonNegativeF64,
    /// Min number of mid symbols
    pub minimum_mid_symbol_count: u16,
    /// Min number of mid symbols if the line is closed
    pub minimum_mid_symbol_count_when_closed: u16,
    /// Whether to always show at least 1 mid symbol
    pub show_at_least_one_mid_symbol: bool,
    /// How to place the mid symbols
    pub mid_symbol_placement: MidSymbolPlacement,
    /// The mid symbol point symbol
    pub mid_symbol: PointSymbol,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CapStyle {
    #[default]
    Flat = 0,
    Round = 1,
    Square = 2,
    //PointedCap = 3, // deprecated, replace with default
}

impl FromStr for CapStyle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "0" => Ok(CapStyle::Flat),
            "1" => Ok(CapStyle::Round),
            "2" => Ok(CapStyle::Square),
            _ => Err(Error::SymbolError),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum JoinStyle {
    Bevel = 0,
    #[default]
    Miter = 1,
    Round = 2,
}

impl FromStr for JoinStyle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "0" => Ok(JoinStyle::Bevel),
            "1" => Ok(JoinStyle::Miter),
            "2" => Ok(JoinStyle::Round),
            _ => Err(Error::SymbolError),
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, Default)]
pub enum MidSymbolPlacement {
    /// Mid symbols on every dash
    #[default]
    CenterOfDash = 0,
    /// Mid symbols on the center of a dash group
    CenterOfDashGroup = 1,
    /// Mid symbols on the main gap (i.e. not between dashes in a group)
    CenterOfGap = 2,
    //NoMidSymbols = 99,
}

impl FromStr for MidSymbolPlacement {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "0" => Ok(MidSymbolPlacement::CenterOfDash),
            "1" => Ok(MidSymbolPlacement::CenterOfDashGroup),
            "2" => Ok(MidSymbolPlacement::CenterOfGap),
            _ => Err(Error::SymbolError),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BorderStyle {
    SymmetricBorder {
        both: LineSymbolBorder,
    },
    AsymmetricBorder {
        left: LineSymbolBorder,
        right: LineSymbolBorder,
    },
}

#[derive(Debug, Clone)]
pub struct LineSymbolBorder {
    pub color: SymbolColor,
    pub width: NonNegativeF64,
    pub shift: NonNegativeF64,
    pub dashed: Option<BorderDash>,
}

impl LineSymbolBorder {
    fn parse(element: &BytesStart<'_>, color_set: &ColorSet) -> Result<Self> {
        let color_index = try_get_attr(element, "color").unwrap_or(-1);
        let color = SymbolColor::from_index(color_index, color_set);
        let width = NonNegativeF64::from_file_value(try_get_attr(element, "width").unwrap_or(0));
        let shift = NonNegativeF64::from_file_value(try_get_attr(element, "shift").unwrap_or(0));
        let is_dashed = try_get_attr(element, "dashed").unwrap_or(false);
        let dashed = if is_dashed {
            Some(BorderDash {
                dash_length: NonNegativeF64::from_file_value(
                    try_get_attr(element, "dash_length").unwrap_or(0),
                ),
                break_length: NonNegativeF64::from_file_value(
                    try_get_attr(element, "break_length").unwrap_or(0),
                ),
            })
        } else {
            None
        };
        Ok(LineSymbolBorder {
            color,
            width,
            shift,
            dashed,
        })
    }

    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>, color_set: &ColorSet) -> Result<()> {
        let mut bs = BytesStart::new("border").with_attributes([
            (
                "color",
                self.color.get_priority(color_set).to_string().as_str(),
            ),
            ("width", self.width.to_file_value()?.to_string().as_str()),
            ("shift", self.shift.to_file_value()?.to_string().as_str()),
        ]);
        if let Some(border_dash) = &self.dashed {
            bs.push_attribute(("dashed", "true"));
            bs.push_attribute((
                "break_length",
                border_dash
                    .break_length
                    .to_file_value()?
                    .to_string()
                    .as_str(),
            ));
            bs.push_attribute((
                "dash_length",
                border_dash
                    .dash_length
                    .to_file_value()?
                    .to_string()
                    .as_str(),
            ));
        }
        writer.write_event(Event::Empty(bs))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BorderDash {
    pub dash_length: NonNegativeF64,
    pub break_length: NonNegativeF64,
}

#[derive(Debug, Clone)]
pub enum DashStyle {
    Dashed {
        dash_length: NonNegativeF64,
        break_length: NonNegativeF64,
        dash_group: GroupDashes,
    },
    NotDashed {
        segment_length: NonNegativeF64,
        end_length: NonNegativeF64,
    },
}

impl Default for DashStyle {
    fn default() -> Self {
        DashStyle::NotDashed {
            segment_length: NonNegativeF64::clamped_from(4.),
            end_length: NonNegativeF64::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GroupDashes {
    Grouped {
        /// allowed to be in range 2..=4
        dashes_in_group: u8,
        in_group_break_length: NonNegativeF64,
    },
    UnGrouped {
        half_outer_dashes: bool,
    },
}

impl Default for GroupDashes {
    fn default() -> Self {
        GroupDashes::UnGrouped {
            half_outer_dashes: false,
        }
    }
}

impl LineSymbol {
    pub fn get_name(&self) -> &str {
        &self.common.name
    }
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<LineSymbol> {
        let mut common = attributes;
        let mut color = SymbolColor::NoColor;
        let mut line_width = NonNegativeF64::default();
        let mut minimum_length = NonNegativeF64::default();
        let mut start_offset = NonNegativeF64::default();
        let mut end_offset = NonNegativeF64::default();
        let mut cap_style = CapStyle::default();
        let mut join_style = JoinStyle::default();
        let mut dashed = false;
        let mut segment_length = 4000;
        let mut end_length = 0;
        let mut dash_length = 4000;
        let mut break_length = 1000;
        let mut dashes_in_group = 1;
        let mut in_group_break_length = 500;
        let mut half_outer_dashes = false;
        let mut show_at_least_one_symbol = true;
        let mut minimum_mid_symbol_count = 0;
        let mut minimum_mid_symbol_count_when_closed = 0;
        let mut mid_symbols_per_spot = 1;
        let mut mid_symbol_distance = 0;
        let mut mid_symbol_placement = MidSymbolPlacement::default();
        let mut suppress_dash_symbol_at_ends = false;
        let mut scale_dash_symbol = true;

        let mut start_symbol = None;
        let mut mid_symbol_point = None;
        let mut end_symbol = None;
        let mut dash_symbol_point = None;
        let mut border = None;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"description" => {
                        if let Event::Text(text) = reader.read_event_into(&mut buf)? {
                            common.description = String::from_utf8(text.to_vec())?;
                        }
                    }
                    b"line_symbol" => {
                        let color_index = try_get_attr(&e, "color").unwrap_or(-1);
                        color = SymbolColor::from_index(color_index, color_set);
                        line_width = NonNegativeF64::from_file_value(
                            try_get_attr(&e, "line_width").unwrap_or(0),
                        );
                        minimum_length = NonNegativeF64::from_file_value(
                            try_get_attr(&e, "minimum_length").unwrap_or(0),
                        );
                        join_style = try_get_attr(&e, "join_style").unwrap_or_default();
                        cap_style = try_get_attr(&e, "cap_style").unwrap_or_default();
                        start_offset = NonNegativeF64::from_file_value(
                            try_get_attr(&e, "start_offset").unwrap_or(0),
                        );
                        end_offset = NonNegativeF64::from_file_value(
                            try_get_attr(&e, "end_offset").unwrap_or(0),
                        );
                        dashed = try_get_attr(&e, "dashed").unwrap_or(false);
                        segment_length = try_get_attr(&e, "segment_length").unwrap_or(4000);
                        end_length = try_get_attr(&e, "end_length").unwrap_or(0);
                        dash_length = try_get_attr(&e, "dash_length").unwrap_or(4000);
                        break_length = try_get_attr(&e, "break_length").unwrap_or(1000);
                        dashes_in_group = try_get_attr(&e, "dashes_in_group").unwrap_or(1);
                        in_group_break_length =
                            try_get_attr(&e, "in_group_break_length").unwrap_or(500);
                        half_outer_dashes = try_get_attr(&e, "half_outer_dashes").unwrap_or(false);
                        show_at_least_one_symbol =
                            try_get_attr(&e, "show_at_least_one_symbol").unwrap_or(false);
                        minimum_mid_symbol_count =
                            try_get_attr(&e, "minimum_mid_symbol_count").unwrap_or(0);
                        minimum_mid_symbol_count_when_closed =
                            try_get_attr(&e, "minimum_mid_symbol_count_when_closed").unwrap_or(0);
                        mid_symbols_per_spot =
                            try_get_attr(&e, "mid_symbols_per_spot").unwrap_or(1);
                        mid_symbol_distance = try_get_attr(&e, "mid_symbol_distance").unwrap_or(0);
                        mid_symbol_placement =
                            try_get_attr(&e, "mid_symbol_placement").unwrap_or_default();
                        suppress_dash_symbol_at_ends =
                            try_get_attr(&e, "suppress_dash_symbol_at_ends").unwrap_or(false);
                        scale_dash_symbol = try_get_attr(&e, "scale_dash_symbol").unwrap_or(true);
                    }
                    b"start_symbol" => {
                        start_symbol = Self::parse_sub_point_symbol(reader, color_set)?;
                    }
                    b"mid_symbol" => {
                        mid_symbol_point = Self::parse_sub_point_symbol(reader, color_set)?;
                    }
                    b"end_symbol" => {
                        end_symbol = Self::parse_sub_point_symbol(reader, color_set)?;
                    }
                    b"dash_symbol" => {
                        dash_symbol_point = Self::parse_sub_point_symbol(reader, color_set)?;
                    }
                    b"borders" => {
                        border = Self::parse_borders(reader, &e, color_set)?;
                    }
                    _ => {}
                },
                Event::Empty(e) => {
                    if e.local_name().as_ref() == b"icon"
                        && let Some(src) = try_get_attr(&e, "src")
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
                        "Unexpected EOF in LineSymbol parsing".to_string(),
                    ));
                }
                _ => {}
            }
        }

        let dash_style = if dashed {
            let dash_group = if dashes_in_group >= 2 {
                GroupDashes::Grouped {
                    dashes_in_group,
                    in_group_break_length: NonNegativeF64::from_file_value(in_group_break_length),
                }
            } else {
                GroupDashes::UnGrouped { half_outer_dashes }
            };
            DashStyle::Dashed {
                dash_length: NonNegativeF64::from_file_value(dash_length),
                break_length: NonNegativeF64::from_file_value(break_length),
                dash_group,
            }
        } else {
            DashStyle::NotDashed {
                segment_length: NonNegativeF64::from_file_value(segment_length),
                end_length: NonNegativeF64::from_file_value(end_length),
            }
        };

        let mid_symbol = mid_symbol_point.map(|ps| MidSymbol {
            mid_symbols_per_spot,
            mid_symbol_distance: NonNegativeF64::from_file_value(mid_symbol_distance),
            minimum_mid_symbol_count,
            minimum_mid_symbol_count_when_closed,
            show_at_least_one_mid_symbol: show_at_least_one_symbol,
            mid_symbol_placement,
            mid_symbol: ps,
        });

        let dash_symbol = dash_symbol_point.map(|ps| DashSymbol {
            suppress_dash_symbol_at_ends,
            scale_dash_symbol,
            dash_symbol: ps,
        });

        Ok(LineSymbol {
            common,
            border,
            start_symbol,
            mid_symbol,
            end_symbol,
            dash_symbol,
            color,
            line_width,
            minimum_length,
            start_offset,
            end_offset,
            dash_style,
            cap_style,
            join_style,
        })
    }

    /// Parse a sub point symbol (start/mid/end/dash) wrapped in its container element
    fn parse_sub_point_symbol<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
    ) -> Result<Option<PointSymbol>> {
        let mut buf = Vec::new();
        let mut result = None;
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => {
                    if e.local_name().as_ref() == b"symbol" {
                        let mut sub_common = SymbolCommon::default();
                        for attr in e.attributes().filter_map(std::result::Result::ok) {
                            match attr.key.local_name().as_ref() {
                                b"name" => {
                                    sub_common.name.push_str(&quick_xml::escape::unescape(
                                        std::str::from_utf8(&attr.value)?,
                                    )?);
                                }
                                b"code" => {
                                    sub_common.code =
                                        crate::utils::parse_attr(attr.value).unwrap_or_default();
                                }
                                _ => {}
                            }
                        }
                        result = Some(PointSymbol::parse(reader, color_set, sub_common)?);
                    }
                }
                Event::End(e) => {
                    let name = e.local_name();
                    let n = name.as_ref();
                    if n == b"start_symbol"
                        || n == b"mid_symbol"
                        || n == b"end_symbol"
                        || n == b"dash_symbol"
                    {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF parsing sub point symbol".to_string(),
                    ));
                }
                _ => {}
            }
        }
        Ok(result)
    }

    /// Parse borders element
    fn parse_borders<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        color_set: &ColorSet,
    ) -> Result<Option<BorderStyle>> {
        let borders_different = try_get_attr(element, "borders_different").unwrap_or(false);
        let mut left = None;
        let mut right = None;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Empty(e) | Event::Start(e) => {
                    if e.local_name().as_ref() == b"border" {
                        let b = LineSymbolBorder::parse(&e, color_set)?;
                        if left.is_none() {
                            left = Some(b);
                        } else {
                            right = Some(b);
                        }
                    }
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == b"borders" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF parsing borders".to_string(),
                    ));
                }
                _ => {}
            }
        }
        match (left, right) {
            (Some(l), Some(r)) if borders_different => {
                Ok(Some(BorderStyle::AsymmetricBorder { left: l, right: r }))
            }
            (Some(both), _) => Ok(Some(BorderStyle::SymmetricBorder { both })),
            _ => Ok(None),
        }
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
        id: Option<usize>,
    ) -> Result<()> {
        let mut bs = BytesStart::new("symbol").with_attributes([
            ("type", "2"),
            ("code", self.common.code.to_string().as_str()),
            (
                "name",
                quick_xml::escape::unescape(self.common.name.as_str())?.as_ref(),
            ),
        ]);
        if let Some(id) = id {
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

        let main_color = self.color.get_priority(color_set);

        let mut bs = BytesStart::new("line_symbol").with_attributes([
            ("color", main_color.to_string().as_str()),
            (
                "line_width",
                self.line_width.to_file_value()?.to_string().as_str(),
            ),
            (
                "minimum_length",
                self.minimum_length.to_file_value()?.to_string().as_str(),
            ),
            ("join_style", (self.join_style as u8).to_string().as_str()),
            ("cap_style", (self.cap_style as u8).to_string().as_str()),
            (
                "start_offset",
                self.start_offset.to_file_value()?.to_string().as_str(),
            ),
            (
                "end_offset",
                self.end_offset.to_file_value()?.to_string().as_str(),
            ),
        ]);

        match &self.dash_style {
            DashStyle::Dashed {
                dash_length,
                break_length,
                dash_group,
            } => {
                bs.push_attribute(("dashed", "true"));
                bs.push_attribute((
                    "dash_length",
                    dash_length.to_file_value()?.to_string().as_str(),
                ));
                bs.push_attribute((
                    "break_length",
                    break_length.to_file_value()?.to_string().as_str(),
                ));
                match dash_group {
                    GroupDashes::Grouped {
                        dashes_in_group,
                        in_group_break_length,
                    } => {
                        bs.push_attribute((
                            "dashes_in_group",
                            dashes_in_group.to_string().as_str(),
                        ));
                        bs.push_attribute((
                            "in_group_break_length",
                            in_group_break_length.to_file_value()?.to_string().as_str(),
                        ));
                    }
                    GroupDashes::UnGrouped { half_outer_dashes } => {
                        if *half_outer_dashes {
                            bs.push_attribute(("half_outer_dashes", "true"));
                        }
                    }
                }
            }
            DashStyle::NotDashed {
                segment_length,
                end_length,
            } => {
                bs.push_attribute((
                    "segment_length",
                    segment_length.to_file_value()?.to_string().as_str(),
                ));
                bs.push_attribute((
                    "end_length",
                    end_length.to_file_value()?.to_string().as_str(),
                ));
            }
        }
        if let Some(mid_symbol) = &self.mid_symbol {
            bs.push_attribute((
                "mid_symbol_distance",
                mid_symbol
                    .mid_symbol_distance
                    .to_file_value()?
                    .to_string()
                    .as_str(),
            ));
            bs.push_attribute((
                "mid_symbol_placement",
                (mid_symbol.mid_symbol_placement as u8).to_string().as_str(),
            ));
            bs.push_attribute((
                "mid_symbols_per_spot",
                mid_symbol.mid_symbols_per_spot.to_string().as_str(),
            ));
            bs.push_attribute((
                "minimum_mid_symbol_count",
                mid_symbol.minimum_mid_symbol_count.to_string().as_str(),
            ));
            bs.push_attribute((
                "minimum_mid_symbol_count_when_closed",
                mid_symbol
                    .minimum_mid_symbol_count_when_closed
                    .to_string()
                    .as_str(),
            ));
            if mid_symbol.show_at_least_one_mid_symbol {
                bs.push_attribute(("show_at_least_one_mid_symbol", "true"));
            }
        }
        if let Some(dash_symbol) = &self.dash_symbol {
            if dash_symbol.scale_dash_symbol {
                bs.push_attribute(("scale_dash_symbol", "true"));
            }
            if dash_symbol.suppress_dash_symbol_at_ends {
                bs.push_attribute(("suppress_dash_symbol_at_ends", "true"));
            }
        }
        writer.write_event(Event::Start(bs))?;

        if let Some(border) = &self.border {
            match border {
                BorderStyle::SymmetricBorder { both } => {
                    writer.write_event(Event::Start(BytesStart::new("borders")))?;
                    both.write(writer, color_set)?;
                    writer.write_event(Event::End(BytesEnd::new("borders")))?;
                }
                BorderStyle::AsymmetricBorder { left, right } => {
                    writer.write_event(Event::Start(
                        BytesStart::new("borders").with_attributes([("borders_different", "true")]),
                    ))?;
                    left.write(writer, color_set)?;
                    right.write(writer, color_set)?;
                    writer.write_event(Event::End(BytesEnd::new("borders")))?;
                }
            }
            writer.write_event(Event::End(BytesEnd::new("borders")))?;
        }
        if let Some(start_symbol) = &self.start_symbol {
            writer.write_event(Event::Start(BytesStart::new("start_symbol")))?;
            start_symbol.write(writer, color_set, None)?;
            writer.write_event(Event::End(BytesEnd::new("start_symbol")))?;
        }
        if let Some(mid_symbol) = &self.mid_symbol {
            writer.write_event(Event::Start(BytesStart::new("mid_symbol")))?;
            mid_symbol.mid_symbol.write(writer, color_set, None)?;
            writer.write_event(Event::End(BytesEnd::new("mid_symbol")))?;
        }
        if let Some(dash_symbol) = &self.dash_symbol {
            writer.write_event(Event::Start(BytesStart::new("dash_symbol")))?;
            dash_symbol.dash_symbol.write(writer, color_set, None)?;
            writer.write_event(Event::End(BytesEnd::new("dash_symbol")))?;
        }
        if let Some(end_symbol) = &self.end_symbol {
            writer.write_event(Event::Start(BytesStart::new("end_symbol")))?;
            end_symbol.write(writer, color_set, None)?;
            writer.write_event(Event::End(BytesEnd::new("end_symbol")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("line_symbol")))?;

        if let Some(icon) = &self.common.custom_icon {
            writer.write_event(Event::Empty(
                BytesStart::new("icon").with_attributes([("src", icon.as_str())]),
            ))?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbol")))?;
        Ok(())
    }
}
