use std::str::FromStr;

use quick_xml::{Reader, Writer};

use super::{PointSymbol, SymbolCommon};
use crate::{
    Code, Error, Result,
    colors::{ColorSet, SymbolColor},
};

#[derive(Debug, Clone)]
pub enum FillPattern {
    LinePattern {
        angle: f64,
        line_spacing: u32,
        line_offset: u32,
        line_color: SymbolColor,
        line_width: u32,
        rotatable: bool, // stored as flag 16 with the clip options
    },
    PointPattern {
        clip_options: ClippingOption,
        angle: f64,
        line_spacing: u32,
        line_offset: u32,
        offset_along_line: u32,
        point_distance: u32,
        point: PointSymbol,
        rotatable: bool, // stored as flag 16 with the clip options
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ClippingOption {
    #[default]
    ClipElementsAtBoundary = 0,
    NoClippingIfCompletelyInside = 1,
    NoClippingIfCenterInside = 2,
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
            _ => Err(Error::SymbolError),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AreaSymbol {
    pub common: SymbolCommon,

    pub is_rotatable: bool,

    pub color: SymbolColor,
    pub patterns: Vec<FillPattern>,
    pub minimum_area: u32,
}

impl AreaSymbol {
    pub fn get_name(&self) -> &str {
        &self.common.name
    }

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
            minimum_area: 0,
        }
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<AreaSymbol> {
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
