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

/// Widening a typed handle is infallible. Narrowing a [`crate::symbols::SymbolRef`]
/// checks the kind of the referenced symbol.
macro_rules! symbol_ids {
    ($(
        $(#[$meta:meta])*
        $name:ident($($kind:ident),+ $(,)?) => [$($wider:ident),* $(,)?];
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

            impl TryFrom<crate::symbols::SymbolRef<'_>> for $name {
                type Error = crate::Error;

                fn try_from(value: crate::symbols::SymbolRef<'_>) -> crate::Result<Self> {
                    const EXPECTED: &[SymbolKind] = &[$(SymbolKind::$kind),+];
                    let found = value.kind();
                    if EXPECTED.contains(&found) {
                        Ok(Self(value.id().0))
                    } else {
                        Err(crate::Error::SymbolKindMismatch {
                            expected: EXPECTED,
                            found,
                        })
                    }
                }
            }
        )+
    };
}

symbol_ids! {
    /// A handle to a symbol of any kind in a [`crate::symbols::SymbolSet`].
    SymbolId(Line, Area, Point, Text, CombinedArea, CombinedLine) => [];

    /// A handle to the symbol used to render a path object: a line, an area, or
    /// either combined form.
    PathSymbolId(Line, Area, CombinedLine, CombinedArea) => [SymbolId];

    /// A handle to the symbol used to render a line object: a
    /// [`crate::symbols::LineSymbol`] or a
    /// [`crate::symbols::CombinedLineSymbol`].
    LinePathSymbolId(Line, CombinedLine) => [PathSymbolId, SymbolId];

    /// A handle to the symbol used to render an area object: an
    /// [`crate::symbols::AreaSymbol`] or a
    /// [`crate::symbols::CombinedAreaSymbol`].
    AreaPathSymbolId(Area, CombinedArea) => [PathSymbolId, SymbolId];

    /// A handle to a [`crate::symbols::PointSymbol`].
    PointSymbolId(Point) => [SymbolId];

    /// A handle to a [`crate::symbols::TextSymbol`].
    TextSymbolId(Text) => [SymbolId];

    /// A handle to a [`crate::symbols::LineSymbol`].
    LineSymbolId(Line) => [LinePathSymbolId, PathSymbolId, SymbolId];

    /// A handle to a [`crate::symbols::CombinedLineSymbol`].
    CombinedLineSymbolId(CombinedLine) => [LinePathSymbolId, PathSymbolId, SymbolId];

    /// A handle to an [`crate::symbols::AreaSymbol`].
    AreaSymbolId(Area) => [AreaPathSymbolId, PathSymbolId, SymbolId];

    /// A handle to a [`crate::symbols::CombinedAreaSymbol`].
    CombinedAreaSymbolId(CombinedArea) => [AreaPathSymbolId, PathSymbolId, SymbolId];
}
