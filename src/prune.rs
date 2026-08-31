//! Dropping references to values that are no longer in their set.
//!
//! Removing a symbol or a color leaves every handle that named it dangling.
//! [`crate::Omap::prune_dangling_references`] walks the map and drops those
//! references: a dangling symbol reference becomes `None`, which writes as the
//! format's `-1`; a dangling color becomes
//! [`crate::colors::SymbolColor::NoColor`]; and a dangling combined-symbol or
//! mixed-color component is removed outright.
//!
//! A handle that still names a live value is left exactly as it is, so handles
//! taken before a prune survive it.

use std::collections::HashSet;

use crate::colors::{ColorId, ColorKey, MixedColor, SymbolColor};
use crate::objects::MapObject;
use crate::symbols::{
    AreaOrLineSymbol, AreaSymbol, CombinedAreaSymbol, CombinedLineSymbol, Element, LineSymbol,
    PointSymbol, PublicOrPrivateSymbol, Symbol, SymbolId, SymbolKey, TextSymbol,
};

/// The handles still present in each set.
pub(crate) struct Live {
    pub(crate) symbols: HashSet<SymbolKey>,
    pub(crate) colors: HashSet<ColorKey>,
}

impl Live {
    fn color<T: Copy + Into<ColorId>>(&self, id: T) -> Option<T> {
        self.colors.contains(&id.into().0).then_some(id)
    }

    fn symbol<T: Copy + Into<SymbolId>>(&self, id: T) -> Option<T> {
        self.symbols.contains(&id.into().0).then_some(id)
    }
}

/// Drop every handle a value holds that no longer resolves.
pub(crate) trait Prune {
    fn prune(&mut self, live: &Live);
}

impl Prune for SymbolColor {
    fn prune(&mut self, live: &Live) {
        if let Self::Color(id) = *self {
            *self = match live.color(id) {
                Some(id) => Self::Color(id),
                None => Self::NoColor,
            };
        }
    }
}

impl<T: Prune> Prune for Option<T> {
    fn prune(&mut self, live: &Live) {
        if let Some(value) = self {
            value.prune(live);
        }
    }
}

impl<T: Prune> Prune for Vec<T> {
    fn prune(&mut self, live: &Live) {
        for value in self.iter_mut() {
            value.prune(live);
        }
    }
}

impl<T: Prune> Prune for Box<T> {
    fn prune(&mut self, live: &Live) {
        (**self).prune(live);
    }
}

impl Prune for MixedColor {
    fn prune(&mut self, live: &Live) {
        self.components
            .retain_mut(|component| match live.color(component.color) {
                Some(color) => {
                    component.color = color;
                    true
                }
                None => false,
            });
    }
}

impl Prune for PointSymbol {
    fn prune(&mut self, live: &Live) {
        self.inner_color.prune(live);
        self.outer_color.prune(live);
        self.elements.prune(live);
    }
}

impl Prune for Element {
    fn prune(&mut self, live: &Live) {
        match self {
            Self::Point { symbol, .. } => symbol.prune(live),
            Self::Line { symbol, .. } => symbol.prune(live),
            Self::Area { symbol, .. } => symbol.prune(live),
        }
    }
}

impl Prune for LineSymbol {
    fn prune(&mut self, live: &Live) {
        self.color.prune(live);
        self.border.prune(live);
        self.start_symbol.prune(live);
        self.end_symbol.prune(live);
        if let Some(mid) = &mut self.mid_symbol {
            mid.mid_symbol.prune(live);
        }
        if let Some(dash) = &mut self.dash_symbol {
            dash.dash_symbol.prune(live);
        }
    }
}

impl Prune for crate::symbols::BorderStyle {
    fn prune(&mut self, live: &Live) {
        match self {
            Self::SymmetricBorder { both } => both.color.prune(live),
            Self::AsymmetricBorder { left, right } => {
                left.color.prune(live);
                right.color.prune(live);
            }
        }
    }
}

impl Prune for AreaSymbol {
    fn prune(&mut self, live: &Live) {
        self.color.prune(live);
        for pattern in &mut self.patterns {
            pattern.prune(live);
        }
    }
}

impl Prune for crate::symbols::FillPattern {
    fn prune(&mut self, live: &Live) {
        match self {
            Self::LinePattern { line_color, .. } => line_color.prune(live),
            Self::PointPattern { point, .. } => point.prune(live),
        }
    }
}

impl Prune for TextSymbol {
    fn prune(&mut self, live: &Live) {
        self.color.prune(live);
        if let Some(below) = &mut self.line_below {
            below.color.prune(live);
        }
        match &mut self.framing_mode {
            Some(crate::symbols::FramingMode::LineFraming(framing)) => framing.color.prune(live),
            Some(crate::symbols::FramingMode::ShadowFraming(framing)) => {
                framing.color.prune(live);
            }
            Some(crate::symbols::FramingMode::NoFraming) | None => (),
        }
    }
}

impl Prune for AreaOrLineSymbol {
    fn prune(&mut self, live: &Live) {
        match self {
            Self::Area(symbol) => symbol.prune(live),
            Self::Line(symbol) => symbol.prune(live),
        }
    }
}

impl Prune for CombinedAreaSymbol {
    fn prune(&mut self, live: &Live) {
        self.retain_map_components(|part| match part {
            PublicOrPrivateSymbol::Public(id) => live.symbol(id).map(PublicOrPrivateSymbol::Public),
            PublicOrPrivateSymbol::Private(mut symbol) => {
                symbol.prune(live);
                Some(PublicOrPrivateSymbol::Private(symbol))
            }
        });
    }
}

impl Prune for CombinedLineSymbol {
    fn prune(&mut self, live: &Live) {
        self.retain_map_components(|part| match part {
            PublicOrPrivateSymbol::Public(id) => live.symbol(id).map(PublicOrPrivateSymbol::Public),
            PublicOrPrivateSymbol::Private(mut symbol) => {
                symbol.prune(live);
                Some(PublicOrPrivateSymbol::Private(symbol))
            }
        });
    }
}

impl Prune for Symbol {
    fn prune(&mut self, live: &Live) {
        match self {
            Self::Line(symbol) => symbol.prune(live),
            Self::Area(symbol) => symbol.prune(live),
            Self::Point(symbol) => symbol.prune(live),
            Self::Text(symbol) => symbol.prune(live),
            Self::CombinedArea(symbol) => symbol.prune(live),
            Self::CombinedLine(symbol) => symbol.prune(live),
        }
    }
}

impl Prune for MapObject {
    fn prune(&mut self, live: &Live) {
        fn rewrite<T: Copy + Into<SymbolId>>(slot: &mut Option<T>, live: &Live) {
            *slot = slot.and_then(|id| live.symbol(id));
        }

        match self {
            Self::Point(object) => rewrite(&mut object.symbol, live),
            Self::Line(object) => rewrite(&mut object.symbol, live),
            Self::Area(object) => rewrite(&mut object.symbol, live),
            Self::Text(object) => rewrite(&mut object.symbol, live),
        }
    }
}

impl Prune for crate::colors::Color {
    fn prune(&mut self, live: &Live) {
        match self {
            Self::SpotColor(_) => (),
            Self::MixedColor(color) => color.prune(live),
        }
    }
}
