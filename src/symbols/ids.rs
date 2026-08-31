slotmap::new_key_type! {
    /// The arena slot a symbol handle names.
    pub(crate) struct SymbolKey;
}

/// The kind of a symbol.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SymbolKind {
    /// A [`crate::symbols::LineSymbol`].
    Line,
    /// An [`crate::symbols::AreaSymbol`].
    Area,
    /// A [`crate::symbols::PointSymbol`].
    Point,
    /// A [`crate::symbols::TextSymbol`].
    Text,
    /// A [`crate::symbols::CombinedAreaSymbol`].
    CombinedArea,
    /// A [`crate::symbols::CombinedLineSymbol`].
    CombinedLine,
}

impl SymbolKind {
    /// The `type` attribute the `.omap` format stores for this kind. The format
    /// has no combined line symbol, so both combined kinds write `16`.
    pub(crate) fn type_id(self) -> &'static str {
        match self {
            Self::Point => "1",
            Self::Line => "2",
            Self::Area => "4",
            Self::Text => "8",
            Self::CombinedArea | Self::CombinedLine => "16",
        }
    }
}

/// Widening is a `From`; narrowing needs the set — see
/// [`crate::symbols::SymbolSet::point_id`] and its siblings.
macro_rules! symbol_ids {
    ($(
        $(#[$meta:meta])*
        $name:ident => [$($wider:ident),* $(,)?];
    )+) => {
        $(
            $(#[$meta])*
            ///
            /// Stops resolving once the symbol is removed, and is meaningless
            /// against any other [`crate::Omap`].
            #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            pub struct $name(pub(crate) SymbolKey);

            $(
                impl From<$name> for $wider {
                    fn from(value: $name) -> Self {
                        Self(value.0)
                    }
                }
            )*
        )+
    };
}

symbol_ids! {
    /// A handle to a symbol of any kind in a [`crate::symbols::SymbolSet`].
    SymbolId => [];

    /// A handle to the symbol used to render a path object: a line, an area, or
    /// either combined form.
    PathSymbolId => [SymbolId];

    /// A handle to the symbol used to render a line object: a
    /// [`crate::symbols::LineSymbol`] or a
    /// [`crate::symbols::CombinedLineSymbol`].
    LinePathSymbolId => [PathSymbolId, SymbolId];

    /// A handle to the symbol used to render an area object: an
    /// [`crate::symbols::AreaSymbol`] or a
    /// [`crate::symbols::CombinedAreaSymbol`].
    AreaPathSymbolId => [PathSymbolId, SymbolId];

    /// A handle to a [`crate::symbols::PointSymbol`].
    PointSymbolId => [SymbolId];

    /// A handle to a [`crate::symbols::TextSymbol`].
    TextSymbolId => [SymbolId];

    /// A handle to a [`crate::symbols::LineSymbol`].
    LineSymbolId => [LinePathSymbolId, PathSymbolId, SymbolId];

    /// A handle to a [`crate::symbols::CombinedLineSymbol`].
    CombinedLineSymbolId => [LinePathSymbolId, PathSymbolId, SymbolId];

    /// A handle to an [`crate::symbols::AreaSymbol`].
    AreaSymbolId => [AreaPathSymbolId, PathSymbolId, SymbolId];

    /// A handle to a [`crate::symbols::CombinedAreaSymbol`].
    CombinedAreaSymbolId => [AreaPathSymbolId, PathSymbolId, SymbolId];
}
