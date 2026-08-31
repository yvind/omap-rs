use super::{CombinedLineSymbol, PublicOrPrivateSymbol, SymbolSet};
use crate::{
    colors::ColorId,
    symbols::{Symbol, SymbolId},
};

impl CombinedLineSymbol {
    /// Get the minimum length (in paper dimensions mm) among all line sub-symbols.
    ///
    /// Takes the [`SymbolSet`] that owns the public components. A component no
    /// longer in the set contributes nothing.
    pub fn minimum_length(&self, symbol_set: &SymbolSet) -> f64 {
        let mut min = f64::MAX;
        for part in self.components() {
            let length = match part {
                PublicOrPrivateSymbol::Public(id) => match symbol_set
                    .get(SymbolId::from(*id))
                    .map(|symbol| symbol.symbol())
                {
                    Some(Symbol::Line(symbol)) => symbol.minimum_length.get(),
                    Some(Symbol::CombinedLine(symbol)) => symbol.minimum_length(symbol_set),
                    _ => 0.,
                },
                PublicOrPrivateSymbol::Private(symbol) => symbol.minimum_length.get(),
            };
            if length > 0. {
                min = min.min(length);
            }
        }
        if min == f64::MAX { 0. } else { min }
    }

    /// Every color used in this symbol definition.
    ///
    /// Takes the [`SymbolSet`] that owns the public components. A component no
    /// longer in the set contributes no colors.
    pub fn colors(&self, symbol_set: &SymbolSet) -> Vec<ColorId> {
        self.component_colors(symbol_set)
    }
}
