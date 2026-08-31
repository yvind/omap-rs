use std::ops::Deref;

use super::{Color, ColorId, MixedColorId, SpotColorId};

/// A color together with the handle that names it.
///
/// What every lookup hands back. It dereferences to the [`Color`] for reading
/// and narrows to a typed handle through the `as_*` methods, so a lookup and a
/// narrowing compose in one expression:
///
/// ```
/// # use omap::Omap;
/// # fn example(map: &Omap) {
/// let purple = map.colors.find_by_name("Purple").and_then(|color| color.as_spot());
/// # }
/// ```
///
/// The handle is [`Copy`] and outlives this borrow of the set, so take it with
/// [`ColorRef::id`] or an `as_*` method before mutating the set.
#[derive(Clone, Copy, Debug)]
pub struct ColorRef<'a> {
    id: ColorId,
    color: &'a Color,
}

impl<'a> ColorRef<'a> {
    pub(crate) fn new(id: ColorId, color: &'a Color) -> Self {
        Self { id, color }
    }

    /// The handle naming this color.
    pub fn id(self) -> ColorId {
        self.id
    }

    /// The color, borrowed for as long as the set is.
    pub fn color(self) -> &'a Color {
        self.color
    }

    /// Narrow to a spot color handle, or `None` if this names a mixed color.
    pub fn as_spot(self) -> Option<SpotColorId> {
        matches!(self.color, Color::SpotColor(_)).then_some(SpotColorId(self.id.0))
    }

    /// Narrow to a mixed color handle, or `None` if this names a spot color.
    pub fn as_mixed(self) -> Option<MixedColorId> {
        matches!(self.color, Color::MixedColor(_)).then_some(MixedColorId(self.id.0))
    }
}

impl Deref for ColorRef<'_> {
    type Target = Color;

    fn deref(&self) -> &Self::Target {
        self.color
    }
}
