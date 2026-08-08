use crate::arena::RawId;
use crate::{Error, Result};

macro_rules! typed_color_id {
    ($name:ident, $variant:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub struct $name(pub(crate) RawId);

        impl From<$name> for ColorId {
            fn from(value: $name) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<ColorId> for $name {
            type Error = Error;

            fn try_from(value: ColorId) -> Result<Self> {
                if let ColorId::$variant(id) = value {
                    Ok(id)
                } else {
                    Err(Error::ColorConversionError)
                }
            }
        }
    };
}

/// A handle to a color of any kind in a [`crate::colors::ColorSet`].
///
/// Handles are [`Copy`] and compare by identity. A handle stays valid while the
/// color it names is in the set, including across reordering, and stops
/// resolving once that color is removed. It is meaningless against any other
/// [`crate::Omap`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ColorId {
    /// A handle to a spot color.
    Spot(SpotColorId),
    /// A handle to a mixed color.
    Mixed(MixedColorId),
}

typed_color_id!(
    SpotColorId,
    Spot,
    "A handle to a [`crate::colors::SpotColor`]."
);
typed_color_id!(
    MixedColorId,
    Mixed,
    "A handle to a [`crate::colors::MixedColor`]."
);

impl ColorId {
    pub(crate) fn raw(self) -> RawId {
        match self {
            Self::Spot(id) => id.0,
            Self::Mixed(id) => id.0,
        }
    }
}
