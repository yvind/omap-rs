mod area_symbol;
mod line_symbol;
mod point_symbol;
mod symbol;
mod text_symbol;

pub use area_symbol::AreaSymbol;
pub use line_symbol::LineSymbol;
pub use point_symbol::PointSymbol;
pub use symbol::Symbol;
pub use text_symbol::TextSymbol;

use crate::writer::Scale;

/// trait defining the three functions all symbol types must have
pub trait SymbolTrait {
    /// minimum size of an object with the symbol at the scale
    fn min_size(&self, _scale: Scale) -> f64 {
        0.
    }

    /// some symbols are rotatable or their pattern are rotatable
    fn is_rotatable(&self) -> bool {
        false
    }

    /// the id of the symbol in the symbol_x.txt files
    fn id(&self) -> u8;
}
