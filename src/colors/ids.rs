slotmap::new_key_type! {
    /// The arena slot a color handle names.
    pub(crate) struct ColorKey;
}

/// The kind of a color.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColorKind {
    /// A [`crate::colors::SpotColor`].
    Spot,
    /// A [`crate::colors::MixedColor`].
    Mixed,
}

/// Widening a typed handle is infallible. Narrowing a [`crate::colors::ColorRef`]
/// checks the kind of the referenced color.
macro_rules! color_ids {
    ($(
        $(#[$meta:meta])*
        $name:ident($($kind:ident),+ $(,)?) => [$($wider:ident),* $(,)?];
    )+) => {
        $(
            $(#[$meta])*
            ///
            /// Stops resolving once the color is removed, and is meaningless
            /// against any other [`crate::Omap`].
            #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            pub struct $name(pub(crate) ColorKey);

            $(
                impl From<$name> for $wider {
                    fn from(value: $name) -> Self {
                        Self(value.0)
                    }
                }
            )*

            impl TryFrom<crate::colors::ColorRef<'_>> for $name {
                type Error = crate::Error;

                fn try_from(value: crate::colors::ColorRef<'_>) -> crate::Result<Self> {
                    const EXPECTED: &[ColorKind] = &[$(ColorKind::$kind),+];
                    let found = value.kind();
                    if EXPECTED.contains(&found) {
                        Ok(Self(value.id().0))
                    } else {
                        Err(crate::Error::ColorKindMismatch {
                            expected: EXPECTED,
                            found,
                        })
                    }
                }
            }
        )+
    };
}

color_ids! {
    /// A handle to a color of any kind in a [`crate::colors::ColorSet`].
    ColorId(Spot, Mixed) => [];

    /// A handle to a [`crate::colors::SpotColor`].
    SpotColorId(Spot) => [ColorId];

    /// A handle to a [`crate::colors::MixedColor`].
    MixedColorId(Mixed) => [ColorId];
}
