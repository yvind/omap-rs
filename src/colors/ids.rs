slotmap::new_key_type! {
    /// The arena slot a color handle names.
    pub(crate) struct ColorKey;
}

macro_rules! color_ids {
    ($(
        $(#[$meta:meta])*
        $name:ident => [$($wider:ident),* $(,)?];
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
        )+
    };
}

color_ids! {
    /// A handle to a color of any kind in a [`crate::colors::ColorSet`].
    ColorId => [];

    /// A handle to a [`crate::colors::SpotColor`].
    SpotColorId => [ColorId];

    /// A handle to a [`crate::colors::MixedColor`].
    MixedColorId => [ColorId];
}
