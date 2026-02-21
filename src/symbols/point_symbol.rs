use quick_xml::{Reader, Writer};

use super::{AreaSymbol, LineSymbol};
use crate::{
    Result,
    colors::{ColorSet, SymbolColor},
    objects::{AreaObject, LineObject, PointObject},
    symbols::SymbolCommon,
};

#[derive(Debug, Clone)]
pub enum Element {
    Point {
        symbol: PointSymbol,
        object: PointObject,
    },
    Line {
        symbol: LineSymbol,
        object: LineObject,
    },
    Area {
        symbol: AreaSymbol,
        object: AreaObject,
    },
}

#[derive(Debug, Clone)]
pub struct PointSymbol {
    pub common: SymbolCommon,

    pub is_rotatable: bool,
    pub elements: Vec<Element>,

    pub inner_color: SymbolColor,
    pub outer_color: SymbolColor,
    pub inner_radius: u32, // in 1/1000 mm
    pub outer_width: u32,  // in 1/1000 mm
}

impl PointSymbol {
    pub fn get_name(&self) -> &str {
        &self.common.name
    }
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<PointSymbol> {
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
