use std::ops::Deref;

use super::{Symbol, SymbolId};

/// A symbol together with the handle that names it.
///
/// What every lookup hands back. It dereferences to the [`Symbol`] for reading
/// and narrows to a typed handle through the `as_*` methods, so a lookup and a
/// narrowing compose in one expression:
///
/// ```
/// # use omap::{Code, Omap};
/// # fn example(map: &Omap) {
/// let gully = map
///     .symbols
///     .find_by_code(Code::new(107, 0, 0))
///     .and_then(|symbol| symbol.as_line_path());
/// # }
/// ```
///
/// The handle is [`Copy`] and outlives this borrow of the set, so take it with
/// [`SymbolRef::id`] or an `as_*` method before mutating the set.
#[derive(Clone, Copy, Debug)]
pub struct SymbolRef<'a> {
    id: SymbolId,
    symbol: &'a Symbol,
}

/// The only place that knows which [`Symbol`] variants a handle type accepts.
macro_rules! narrow {
    ($name:ident, $id:ident, $article:literal, $($pattern:pat_param)|+) => {
        #[doc = concat!("Narrow to ", $article, " handle, or `None` if this names another kind.")]
        pub fn $name(self) -> Option<crate::symbols::$id> {
            matches!(self.symbol, $($pattern)|+).then_some(crate::symbols::$id(self.id.0))
        }
    };
}

impl<'a> SymbolRef<'a> {
    pub(crate) fn new(id: SymbolId, symbol: &'a Symbol) -> Self {
        Self { id, symbol }
    }

    /// The handle naming this symbol.
    pub fn id(self) -> SymbolId {
        self.id
    }

    /// The symbol, borrowed for as long as the set is.
    pub fn symbol(self) -> &'a Symbol {
        self.symbol
    }

    narrow!(
        as_path,
        PathSymbolId,
        "a path",
        Symbol::Line(_) | Symbol::Area(_) | Symbol::CombinedLine(_) | Symbol::CombinedArea(_)
    );
    narrow!(
        as_line_path,
        LinePathSymbolId,
        "a line path",
        Symbol::Line(_) | Symbol::CombinedLine(_)
    );
    narrow!(
        as_area_path,
        AreaPathSymbolId,
        "an area path",
        Symbol::Area(_) | Symbol::CombinedArea(_)
    );
    narrow!(as_point, PointSymbolId, "a point", Symbol::Point(_));
    narrow!(as_text, TextSymbolId, "a text", Symbol::Text(_));
    narrow!(as_line, LineSymbolId, "a line", Symbol::Line(_));
    narrow!(as_area, AreaSymbolId, "an area", Symbol::Area(_));
    narrow!(
        as_combined_line,
        CombinedLineSymbolId,
        "a combined line",
        Symbol::CombinedLine(_)
    );
    narrow!(
        as_combined_area,
        CombinedAreaSymbolId,
        "a combined area",
        Symbol::CombinedArea(_)
    );
}

impl Deref for SymbolRef<'_> {
    type Target = Symbol;

    fn deref(&self) -> &Self::Target {
        self.symbol
    }
}
