use std::{cell::RefCell, fmt::Debug, rc::Weak};

use quick_xml::{Reader, Writer};

use super::{AreaSymbol, LineSymbol, PubOrPrivSymbol, Symbol, SymbolCommon, SymbolSet};
use crate::{Result, colors::ColorSet};

#[derive(Debug, Clone)]
pub struct CombinedAreaSymbol {
    pub common: SymbolCommon,
    pub parts: Vec<PubOrPrivSymbol<WeakPathSymbol, PathSymbol>>,
}

#[derive(Debug, Clone)]
pub enum PathSymbol {
    Area(AreaSymbol),
    Line(LineSymbol),
}

#[derive(Debug, Clone)]
pub enum WeakPathSymbol {
    Area(Weak<RefCell<AreaSymbol>>),
    Line(Weak<RefCell<LineSymbol>>),
}

impl WeakPathSymbol {
    pub fn upgrade(&self) -> Option<Symbol> {
        match self {
            WeakPathSymbol::Area(weak) => weak.upgrade().map(|a| Symbol::Area(a)),
            WeakPathSymbol::Line(weak) => weak.upgrade().map(|a| Symbol::Line(a)),
        }
    }
}

impl CombinedAreaSymbol {
    pub fn get_name(&self) -> &str {
        &self.common.name
    }
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<(CombinedAreaSymbol, Vec<usize>)> {
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
