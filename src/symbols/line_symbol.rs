use std::str::FromStr;

use quick_xml::{Reader, Writer};

use super::{PointSymbol, SymbolCommon};
use crate::{
    Error, Result,
    colors::{ColorSet, SymbolColor},
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
    pub line_width: u32,
    pub minimum_length: u32,
    pub start_offset: u32,
    pub end_offset: u32,

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
    pub mid_symbols_per_spot: u16,
    pub mid_symbol_distance: u32,
    pub minimum_mid_symbol_count: u16,
    pub minimum_mid_symbol_count_when_closed: u16,
    pub show_at_least_one_mid_symbol: bool,
    pub mid_symbol_placement: MidSymbolPlacement,
    pub mid_symbol: PointSymbol,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CapStyle {
    #[default]
    FlatCap = 0,
    RoundCap = 1,
    SquareCap = 2,
    //PointedCap = 3, // deprecated, replace with default
}

impl FromStr for CapStyle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "0" => Ok(CapStyle::FlatCap),
            "1" => Ok(CapStyle::RoundCap),
            "2" => Ok(CapStyle::SquareCap),
            _ => Err(Error::SymbolError),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum JoinStyle {
    BevelJoin = 0,
    #[default]
    MiterJoin = 1,
    RoundJoin = 2,
}

impl FromStr for JoinStyle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "0" => Ok(JoinStyle::BevelJoin),
            "1" => Ok(JoinStyle::MiterJoin),
            "2" => Ok(JoinStyle::RoundJoin),
            _ => Err(Error::SymbolError),
        }
    }
}

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
    ASymmetricBorder {
        left: LineSymbolBorder,
        right: LineSymbolBorder,
    },
}

#[derive(Debug, Clone)]
pub struct LineSymbolBorder {
    pub color: SymbolColor,
    pub width: u32,
    pub shift: u32,
    pub dashed: Option<BorderDash>,
}

#[derive(Debug, Clone)]
pub struct BorderDash {
    pub dash_length: u32,
    pub break_length: u32,
}

#[derive(Debug, Clone)]
pub enum DashStyle {
    Dashed {
        dash_length: u32,
        break_length: u32,
        dash_group: GroupDashes,
    },
    NotDashed {
        segment_length: u32,
        end_length: u32,
    },
}

impl Default for DashStyle {
    fn default() -> Self {
        DashStyle::NotDashed {
            segment_length: 4000,
            end_length: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GroupDashes {
    Grouped {
        /// allowed to be in range 2..=4
        dashes_in_group: u8,
        in_group_break_length: u32,
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
        todo!()
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
        id: usize,
    ) -> Result<()> {
        writer.write_all(
            format!(
                "<symbol type=\"2\" id\"{id}\" code=\"{}\" name=\"{}\" is_helper_symbol=\"{}\" is_hidden=\"{}\" is_protected=\"{}\">",
                self.code,
                quick_xml::escape::escape(&self.name),
                self.is_helper_symbol,
                self.is_hidden,
                self.is_protected,
            )
            .as_bytes(),
        )?;
        if !self.description.is_empty() {
            writer.write_all(
                format!(
                    "<description>{}</description>",
                    quick_xml::escape::escape(&self.description)
                )
                .as_bytes(),
            )?;
        }
        writer.write_all(
            format!(
                "<line_symbol color=\"{}\" line_width=\"{}\" minimum_length=\"{}\" join_style=\"{}\" cap_style=\"{}\" start_offset=\"{}\" end_offset=\"{}\">",
                self.color.get_priority(color_set),
                self.line_width,
                self.minimum_length,
                self.join_style as u8,
                self.cap_style as u8,
                self.start_offset,
                self.end_offset
            ).as_bytes()
        )?;

        writer.write_all(b"</line_symbol>")?;
        if let Some(icon) = &self.icon {
            writer.write_all(format!("\n<icon src=\"{}\"/>", icon).as_bytes())?;
        }
        writer.write_all(b"</symbol>\n")?;
        Ok(())
    }
}
