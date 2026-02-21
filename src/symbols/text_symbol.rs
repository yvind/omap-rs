use geo_types::Coord;
use quick_xml::{Reader, Writer};

use super::SymbolCommon;
use crate::{
    Error, Result,
    colors::{ColorSet, SymbolColor},
};

#[derive(Debug, Clone, Default)]
enum FramingMode {
    #[default]
    NoFraming,
    LineFraming(LineFraming),
    ShadowFraming(ShadowFraming),
}

impl FramingMode {
    pub fn get_id(&self) -> u8 {
        match self {
            FramingMode::NoFraming => 0,
            FramingMode::LineFraming(_) => 1,
            FramingMode::ShadowFraming(_) => 2,
        }
    }
}

#[derive(Debug, Clone)]
struct LineFraming {
    color: SymbolColor,
    framing_line_half_width: i32,
}

#[derive(Debug, Clone)]
struct ShadowFraming {
    color: SymbolColor,
    framing_shadow_offset: Coord<i32>,
}

#[derive(Debug, Clone)]
struct LineBelow {
    color: SymbolColor,
    width: i32,
    distance: i32,
}

#[derive(Debug, Clone)]
pub struct TextSymbol {
    pub common: SymbolCommon,

    pub font_family: String,
    /// Should not be more than 3 chars long
    pub icon_text: String,

    pub color: SymbolColor,

    // OCD compat
    pub custom_tabs: Vec<i32>,
    pub line_below: Option<LineBelow>,

    /// default tab interval length in text coordinates
    pub tab_interval: f64,
    pub line_spacing: f32,      // as factor of original line spacing
    pub character_spacing: f32, // as a factor of the space character width
    pub font_size: u32, // this defines the font size in 1/1000 mm. How big the letters really are depends on the design of the font though
    pub paragraph_spacing: i32, // in mm
    pub framing_mode: Option<FramingMode>,

    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub kerning: bool,
}

impl TextSymbol {
    pub fn get_name(&self) -> &str {
        &self.common.name
    }
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<TextSymbol> {
        todo!()
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
        index: usize,
    ) -> Result<()> {
        todo!()
    }
}
