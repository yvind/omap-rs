use std::{cell::RefCell, rc::Weak};

use quick_xml::{Reader, Writer};

use super::{LineSymbol, PubOrPrivSymbol, SymbolCommon, SymbolSet};
use crate::{Result, colors::ColorSet};

#[derive(Debug, Clone)]
pub struct CombinedLineSymbol {
    pub common: SymbolCommon,
    pub parts: Vec<PubOrPrivSymbol<Weak<RefCell<LineSymbol>>, LineSymbol>>,
}

impl CombinedLineSymbol {
    pub fn get_name(&self) -> &str {
        &self.common.name
    }
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<(CombinedLineSymbol, Vec<usize>)> {
        todo!()
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
        color_set: &ColorSet,
        index: usize,
    ) -> Result<()> {
        todo!()
    }
}
