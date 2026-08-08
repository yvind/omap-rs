//! Renumbering handles so that a handle's slot index is also its position.
//!
//! Serialization writes each set as a plain ordered list and each handle as a
//! bare integer, matching what the `.omap` format stores. That is only correct
//! while every handle's slot index equals its position, which holds for any map
//! that has never had a symbol or colour removed. [`crate::Omap::compact`]
//! restores the property, dropping references to anything already removed.

use std::collections::HashMap;

use crate::arena::RawId;
use crate::colors::{ColorId, MixedColor, MixedColorId, SpotColorId, SymbolColor};
use crate::objects::MapObject;
use crate::symbols::{
    AreaOrLineSymbol, AreaSymbol, CombinedAreaSymbol, CombinedLineSymbol, Element,
    LinePathSymbolId, LineSymbol, PathSymbolId, PointSymbol, PublicOrPrivateSymbol, Symbol,
    SymbolId, TextSymbol,
};

/// The old-handle to new-handle mappings produced by compacting the two sets.
pub(crate) struct Remap {
    pub(crate) symbols: HashMap<RawId, RawId>,
    pub(crate) colors: HashMap<RawId, RawId>,
}

impl Remap {
    fn color(&self, id: ColorId) -> Option<ColorId> {
        let raw = *self.colors.get(&id.raw())?;
        Some(match id {
            ColorId::Spot(_) => ColorId::Spot(SpotColorId(raw)),
            ColorId::Mixed(_) => ColorId::Mixed(MixedColorId(raw)),
        })
    }

    fn spot_color(&self, id: SpotColorId) -> Option<SpotColorId> {
        self.colors.get(&id.0).copied().map(SpotColorId)
    }

    fn symbol(&self, id: SymbolId) -> Option<SymbolId> {
        use crate::symbols as s;
        let raw = *self.symbols.get(&id.raw())?;
        Some(match id {
            SymbolId::Line(_) => SymbolId::Line(s::LineSymbolId(raw)),
            SymbolId::Area(_) => SymbolId::Area(s::AreaSymbolId(raw)),
            SymbolId::Point(_) => SymbolId::Point(s::PointSymbolId(raw)),
            SymbolId::Text(_) => SymbolId::Text(s::TextSymbolId(raw)),
            SymbolId::CombinedArea(_) => SymbolId::CombinedArea(s::CombinedAreaSymbolId(raw)),
            SymbolId::CombinedLine(_) => SymbolId::CombinedLine(s::CombinedLineSymbolId(raw)),
        })
    }
}

/// Rewrite every handle a value holds, dropping any that no longer resolve.
pub(crate) trait Compact {
    fn compact(&mut self, remap: &Remap);
}

impl Compact for SymbolColor {
    fn compact(&mut self, remap: &Remap) {
        if let Self::Color(id) = *self {
            *self = match remap.color(id) {
                Some(id) => Self::Color(id),
                None => Self::NoColor,
            };
        }
    }
}

impl<T: Compact> Compact for Option<T> {
    fn compact(&mut self, remap: &Remap) {
        if let Some(value) = self {
            value.compact(remap);
        }
    }
}

impl<T: Compact> Compact for Vec<T> {
    fn compact(&mut self, remap: &Remap) {
        for value in self.iter_mut() {
            value.compact(remap);
        }
    }
}

impl<T: Compact> Compact for Box<T> {
    fn compact(&mut self, remap: &Remap) {
        (**self).compact(remap);
    }
}

impl Compact for MixedColor {
    fn compact(&mut self, remap: &Remap) {
        self.components
            .retain_mut(|component| match remap.spot_color(component.color) {
                Some(color) => {
                    component.color = color;
                    true
                }
                None => false,
            });
    }
}

impl Compact for PointSymbol {
    fn compact(&mut self, remap: &Remap) {
        self.inner_color.compact(remap);
        self.outer_color.compact(remap);
        self.elements.compact(remap);
    }
}

impl Compact for Element {
    fn compact(&mut self, remap: &Remap) {
        match self {
            Self::Point { symbol, .. } => symbol.compact(remap),
            Self::Line { symbol, .. } => symbol.compact(remap),
            Self::Area { symbol, .. } => symbol.compact(remap),
        }
    }
}

impl Compact for LineSymbol {
    fn compact(&mut self, remap: &Remap) {
        self.color.compact(remap);
        self.border.compact(remap);
        self.start_symbol.compact(remap);
        self.end_symbol.compact(remap);
        if let Some(mid) = &mut self.mid_symbol {
            mid.mid_symbol.compact(remap);
        }
        if let Some(dash) = &mut self.dash_symbol {
            dash.dash_symbol.compact(remap);
        }
    }
}

impl Compact for crate::symbols::BorderStyle {
    fn compact(&mut self, remap: &Remap) {
        match self {
            Self::SymmetricBorder { both } => both.color.compact(remap),
            Self::AsymmetricBorder { left, right } => {
                left.color.compact(remap);
                right.color.compact(remap);
            }
        }
    }
}

impl Compact for AreaSymbol {
    fn compact(&mut self, remap: &Remap) {
        self.color.compact(remap);
        for pattern in &mut self.patterns {
            pattern.compact(remap);
        }
    }
}

impl Compact for crate::symbols::FillPattern {
    fn compact(&mut self, remap: &Remap) {
        match self {
            Self::LinePattern { line_color, .. } => line_color.compact(remap),
            Self::PointPattern { point, .. } => point.compact(remap),
        }
    }
}

impl Compact for TextSymbol {
    fn compact(&mut self, remap: &Remap) {
        self.color.compact(remap);
        if let Some(below) = &mut self.line_below {
            below.color.compact(remap);
        }
        match &mut self.framing_mode {
            Some(crate::symbols::FramingMode::LineFraming(framing)) => framing.color.compact(remap),
            Some(crate::symbols::FramingMode::ShadowFraming(framing)) => {
                framing.color.compact(remap);
            }
            Some(crate::symbols::FramingMode::NoFraming) | None => (),
        }
    }
}

impl Compact for AreaOrLineSymbol {
    fn compact(&mut self, remap: &Remap) {
        match self {
            Self::Area(symbol) => symbol.compact(remap),
            Self::Line(symbol) => symbol.compact(remap),
        }
    }
}

impl Compact for CombinedAreaSymbol {
    fn compact(&mut self, remap: &Remap) {
        self.retain_map_components(|part| match part {
            PublicOrPrivateSymbol::Public(id) => remap
                .symbol(SymbolId::from(id))
                .and_then(|id| PathSymbolId::try_from(id).ok())
                .map(PublicOrPrivateSymbol::Public),
            PublicOrPrivateSymbol::Private(mut symbol) => {
                symbol.compact(remap);
                Some(PublicOrPrivateSymbol::Private(symbol))
            }
        });
    }
}

impl Compact for CombinedLineSymbol {
    fn compact(&mut self, remap: &Remap) {
        self.retain_map_components(|part| match part {
            PublicOrPrivateSymbol::Public(id) => remap
                .symbol(SymbolId::from(id))
                .and_then(|id| LinePathSymbolId::try_from(id).ok())
                .map(PublicOrPrivateSymbol::Public),
            PublicOrPrivateSymbol::Private(mut symbol) => {
                symbol.compact(remap);
                Some(PublicOrPrivateSymbol::Private(symbol))
            }
        });
    }
}

impl Compact for Symbol {
    fn compact(&mut self, remap: &Remap) {
        match self {
            Self::Line(symbol) => symbol.compact(remap),
            Self::Area(symbol) => symbol.compact(remap),
            Self::Point(symbol) => symbol.compact(remap),
            Self::Text(symbol) => symbol.compact(remap),
            Self::CombinedArea(symbol) => symbol.compact(remap),
            Self::CombinedLine(symbol) => symbol.compact(remap),
        }
    }
}

impl Compact for MapObject {
    fn compact(&mut self, remap: &Remap) {
        fn narrow<T: Copy + Into<SymbolId> + TryFrom<SymbolId>>(
            slot: &mut Option<T>,
            remap: &Remap,
        ) {
            *slot = slot
                .and_then(|id| remap.symbol(id.into()))
                .and_then(|id| T::try_from(id).ok());
        }

        match self {
            Self::Point(object) => narrow(&mut object.symbol, remap),
            Self::Line(object) => narrow(&mut object.symbol, remap),
            Self::Area(object) => narrow(&mut object.symbol, remap),
            Self::Text(object) => narrow(&mut object.symbol, remap),
        }
    }
}

impl Compact for crate::colors::Color {
    fn compact(&mut self, remap: &Remap) {
        match self {
            Self::SpotColor(_) => (),
            Self::MixedColor(color) => color.compact(remap),
        }
    }
}
