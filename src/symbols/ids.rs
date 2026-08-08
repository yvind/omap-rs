use crate::arena::RawId;
use crate::{Error, Result};

macro_rules! typed_symbol_id {
    ($name:ident, $variant:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name(pub(crate) RawId);

        impl From<$name> for SymbolId {
            fn from(value: $name) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<SymbolId> for $name {
            type Error = Error;

            fn try_from(value: SymbolId) -> Result<Self> {
                if let SymbolId::$variant(id) = value {
                    Ok(id)
                } else {
                    Err(Error::SymbolConversionError)
                }
            }
        }
    };
}

/// A handle to a symbol of any kind in a [`crate::symbols::SymbolSet`].
///
/// Handles are [`Copy`] and compare by identity. A handle stays valid while the
/// symbol it names is in the set, including across [`crate::symbols::SymbolSet::sort`],
/// and stops resolving once that symbol is removed. It is meaningless against
/// any other [`crate::Omap`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SymbolId {
    /// A handle to a line symbol.
    Line(LineSymbolId),
    /// A handle to an area symbol.
    Area(AreaSymbolId),
    /// A handle to a point symbol.
    Point(PointSymbolId),
    /// A handle to a text symbol.
    Text(TextSymbolId),
    /// A handle to a combined area symbol.
    CombinedArea(CombinedAreaSymbolId),
    /// A handle to a combined line symbol.
    CombinedLine(CombinedLineSymbolId),
}

typed_symbol_id!(
    LineSymbolId,
    Line,
    "A handle to a [`crate::symbols::LineSymbol`]."
);
typed_symbol_id!(
    AreaSymbolId,
    Area,
    "A handle to an [`crate::symbols::AreaSymbol`]."
);
typed_symbol_id!(
    PointSymbolId,
    Point,
    "A handle to a [`crate::symbols::PointSymbol`]."
);
typed_symbol_id!(
    TextSymbolId,
    Text,
    "A handle to a [`crate::symbols::TextSymbol`]."
);
typed_symbol_id!(
    CombinedAreaSymbolId,
    CombinedArea,
    "A handle to a [`crate::symbols::CombinedAreaSymbol`]."
);
typed_symbol_id!(
    CombinedLineSymbolId,
    CombinedLine,
    "A handle to a [`crate::symbols::CombinedLineSymbol`]."
);

impl SymbolId {
    pub(crate) fn raw(self) -> RawId {
        match self {
            Self::Line(id) => id.0,
            Self::Area(id) => id.0,
            Self::Point(id) => id.0,
            Self::Text(id) => id.0,
            Self::CombinedArea(id) => id.0,
            Self::CombinedLine(id) => id.0,
        }
    }
}

/// The symbol used to render a path object: a line, an area, or either
/// combined form.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PathSymbolId {
    /// A standalone line symbol.
    Line(LineSymbolId),
    /// A standalone area symbol.
    Area(AreaSymbolId),
    /// A combined line symbol.
    CombinedLine(CombinedLineSymbolId),
    /// A combined area symbol.
    CombinedArea(CombinedAreaSymbolId),
}

/// The symbol used to render a line object.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LinePathSymbolId {
    /// A standalone line symbol.
    Line(LineSymbolId),
    /// A combined line symbol.
    CombinedLine(CombinedLineSymbolId),
}

/// The symbol used to render an area object.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AreaPathSymbolId {
    /// A standalone area symbol.
    Area(AreaSymbolId),
    /// A combined area symbol.
    CombinedArea(CombinedAreaSymbolId),
}

macro_rules! group_conversions {
    ($group:ident { $($variant:ident($typed:ident)),+ $(,)? }) => {
        $(
            impl From<$typed> for $group {
                fn from(value: $typed) -> Self {
                    Self::$variant(value)
                }
            }
        )+

        impl From<$group> for SymbolId {
            fn from(value: $group) -> Self {
                match value {
                    $($group::$variant(id) => Self::$variant(id),)+
                }
            }
        }

        impl TryFrom<SymbolId> for $group {
            type Error = Error;

            fn try_from(value: SymbolId) -> Result<Self> {
                match value {
                    $(SymbolId::$variant(id) => Ok(Self::$variant(id)),)+
                    _ => Err(Error::SymbolConversionError),
                }
            }
        }
    };
}

group_conversions!(PathSymbolId {
    Line(LineSymbolId),
    Area(AreaSymbolId),
    CombinedLine(CombinedLineSymbolId),
    CombinedArea(CombinedAreaSymbolId),
});
group_conversions!(LinePathSymbolId {
    Line(LineSymbolId),
    CombinedLine(CombinedLineSymbolId),
});
group_conversions!(AreaPathSymbolId {
    Area(AreaSymbolId),
    CombinedArea(CombinedAreaSymbolId),
});

macro_rules! narrow_from_path {
    ($group:ident { $($variant:ident),+ $(,)? }) => {
        impl From<$group> for PathSymbolId {
            fn from(value: $group) -> Self {
                match value {
                    $($group::$variant(id) => Self::$variant(id),)+
                }
            }
        }

        impl TryFrom<PathSymbolId> for $group {
            type Error = Error;

            fn try_from(value: PathSymbolId) -> Result<Self> {
                match value {
                    $(PathSymbolId::$variant(id) => Ok(Self::$variant(id)),)+
                    _ => Err(Error::SymbolConversionError),
                }
            }
        }
    };
}

narrow_from_path!(LinePathSymbolId { Line, CombinedLine });
narrow_from_path!(AreaPathSymbolId { Area, CombinedArea });
