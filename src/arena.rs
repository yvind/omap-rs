//! A dense, ordered generational arena.
//!
//! Both symbol and colour sets are ordered collections whose order is part of
//! the `.omap` format, and both reorder after the fact. The arena therefore
//! owns the order as well as the storage, so the two share one invariant, and
//! [`Arena::position`] — the lookup replacing the pointer scans on write — is
//! an O(1) read.

use std::num::NonZeroU32;

/// A handle into an [`Arena`].
///
/// Remains valid while the value it names is in the arena, and stops resolving
/// once that value is removed, even if the slot is later reused.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "u32", from = "u32"))]
pub(crate) struct RawId {
    index: u32,
    generation: NonZeroU32,
}

/// Serialized as the bare slot index, which [`Arena::is_compact`] guarantees is
/// also the value's position — the integer the `.omap` format itself stores.
/// The generation is not serialized: deserialization always rebuilds a fresh
/// arena whose generations are all [`NonZeroU32::MIN`].
#[cfg(feature = "serde")]
impl From<RawId> for u32 {
    fn from(value: RawId) -> Self {
        value.index
    }
}

#[cfg(feature = "serde")]
impl From<u32> for RawId {
    fn from(index: u32) -> Self {
        Self {
            index,
            generation: NonZeroU32::MIN,
        }
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for Arena<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error as _;
        if !self.is_compact() {
            return Err(S::Error::custom(
                "arena must be compacted before serializing; call Omap::compact",
            ));
        }
        self.values.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Arena<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Vec::<T>::deserialize(deserializer)?.into_iter().collect())
    }
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    generation: NonZeroU32,
    position: Option<u32>,
}

pub(crate) struct Arena<T> {
    values: Vec<T>,
    ids: Vec<RawId>,
    slots: Vec<Slot>,
    free: Vec<u32>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self.values).finish()
    }
}

impl<T: Clone> Clone for Arena<T> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            ids: self.ids.clone(),
            slots: self.slots.clone(),
            free: self.free.clone(),
        }
    }
}

impl<T> Arena<T> {
    pub(crate) fn new() -> Self {
        Self {
            values: Vec::new(),
            ids: Vec::new(),
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            ids: Vec::with_capacity(capacity),
            slots: Vec::with_capacity(capacity),
            free: Vec::new(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Does every live handle's slot index equal its position?
    ///
    /// True for any arena that has never had a removal, which covers every
    /// freshly parsed map. Only [`Arena::remove`] can break it, by freeing a
    /// slot that a later push then reuses at a different position.
    pub(crate) fn is_compact(&self) -> bool {
        self.free.is_empty()
            && self.slots.len() == self.values.len()
            && self
                .ids
                .iter()
                .enumerate()
                .all(|(position, id)| id.index as usize == position)
    }

    /// Renumber so that every slot index equals its position, and report the
    /// mapping from old handle to new so references can be remapped. A handle
    /// absent from the returned map named a removed value.
    pub(crate) fn compact(&mut self) -> std::collections::HashMap<RawId, RawId> {
        if self.is_compact() {
            return self.ids.iter().map(|&id| (id, id)).collect();
        }

        // Every slot takes a generation above any handed out so far, so a
        // handle from before compaction fails to resolve rather than aliasing
        // whatever value moved into its old slot.
        let generation = self
            .slots
            .iter()
            .map(|slot| slot.generation)
            .max()
            .unwrap_or(NonZeroU32::MIN)
            .checked_add(1)
            .unwrap_or(NonZeroU32::MAX);

        let mut mapping = std::collections::HashMap::with_capacity(self.ids.len());
        let old = std::mem::take(&mut self.ids);
        self.slots.clear();
        self.free.clear();
        for (position, old_id) in old.into_iter().enumerate() {
            let new_id = RawId {
                index: position as u32,
                generation,
            };
            self.slots.push(Slot {
                generation,
                position: Some(position as u32),
            });
            self.ids.push(new_id);
            let _previous = mapping.insert(old_id, new_id);
        }
        mapping
    }

    fn live_position(&self, id: RawId) -> Option<usize> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.position.map(|position| position as usize)
    }

    /// The position of `id` in write order, or `None` if it names a value that
    /// is no longer in the arena.
    pub(crate) fn position(&self, id: RawId) -> Option<usize> {
        self.live_position(id)
    }

    pub(crate) fn contains(&self, id: RawId) -> bool {
        self.live_position(id).is_some()
    }

    pub(crate) fn get(&self, id: RawId) -> Option<&T> {
        self.values.get(self.live_position(id)?)
    }

    pub(crate) fn get_mut(&mut self, id: RawId) -> Option<&mut T> {
        let position = self.live_position(id)?;
        self.values.get_mut(position)
    }

    pub(crate) fn get_at(&self, position: usize) -> Option<&T> {
        self.values.get(position)
    }

    pub(crate) fn get_at_mut(&mut self, position: usize) -> Option<&mut T> {
        self.values.get_mut(position)
    }

    pub(crate) fn id_at(&self, position: usize) -> Option<RawId> {
        self.ids.get(position).copied()
    }

    /// Append a value in last position.
    pub(crate) fn push(&mut self, value: T) -> RawId {
        let position = self.values.len();
        let id = self.claim_slot(position);
        self.values.push(value);
        self.ids.push(id);
        id
    }

    /// Insert a value at `position`, shifting later values back.
    ///
    /// # Panics
    ///
    /// Panics if `position > self.len()`.
    pub(crate) fn insert(&mut self, position: usize, value: T) -> RawId {
        assert!(position <= self.values.len(), "position out of bounds");
        let id = self.claim_slot(position);
        self.values.insert(position, value);
        self.ids.insert(position, id);
        self.reindex_from(position + 1);
        id
    }

    pub(crate) fn remove(&mut self, id: RawId) -> Option<T> {
        let position = self.live_position(id)?;
        self.remove_at(position)
    }

    pub(crate) fn remove_at(&mut self, position: usize) -> Option<T> {
        if position >= self.values.len() {
            return None;
        }
        let value = self.values.remove(position);
        let id = self.ids.remove(position);
        self.release_slot(id);
        self.reindex_from(position);
        Some(value)
    }

    pub(crate) fn swap(&mut self, first: usize, second: usize) {
        self.values.swap(first, second);
        self.ids.swap(first, second);
        for position in [first, second] {
            if let Some(id) = self.ids.get(position) {
                self.slots[id.index as usize].position = Some(position as u32);
            }
        }
    }

    /// Reorder by `key`, stably, keeping every handle valid.
    pub(crate) fn sort_by_key<K: Ord, F: Fn(&T) -> K>(&mut self, key: F) {
        let mut order: Vec<usize> = (0..self.values.len()).collect();
        order.sort_by_key(|&position| key(&self.values[position]));

        let mut values: Vec<Option<T>> = self.values.drain(..).map(Some).collect();
        let ids = std::mem::take(&mut self.ids);
        for &position in &order {
            #[expect(
                clippy::unwrap_used,
                reason = "`order` is a permutation, so each value is taken exactly once"
            )]
            self.values.push(values[position].take().unwrap());
            self.ids.push(ids[position]);
        }
        self.reindex_from(0);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (RawId, &T)> {
        self.ids.iter().copied().zip(self.values.iter())
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &T> {
        self.values.iter()
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.values.iter_mut()
    }

    fn claim_slot(&mut self, position: usize) -> RawId {
        let position = position as u32;
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.position = Some(position);
            RawId {
                index,
                generation: slot.generation,
            }
        } else {
            let index = self.slots.len() as u32;
            let generation = NonZeroU32::MIN;
            self.slots.push(Slot {
                generation,
                position: Some(position),
            });
            RawId { index, generation }
        }
    }

    fn release_slot(&mut self, id: RawId) {
        let slot = &mut self.slots[id.index as usize];
        slot.position = None;
        match slot.generation.checked_add(1) {
            Some(generation) => {
                slot.generation = generation;
                self.free.push(id.index);
            }
            None => slot.generation = NonZeroU32::MAX,
        }
    }

    fn reindex_from(&mut self, start: usize) {
        for position in start..self.ids.len() {
            let index = self.ids[position].index as usize;
            self.slots[index].position = Some(position as u32);
        }
    }
}

impl<T> FromIterator<T> for Arena<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut arena = Self::with_capacity(iter.size_hint().0);
        for value in iter {
            let _id = arena.push(value);
        }
        arena
    }
}

#[expect(clippy::unwrap_used, reason = "tests assert on known-live handles")]
#[cfg(test)]
mod tests {
    use super::Arena;

    fn arena_of(values: &[&str]) -> Arena<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn order(arena: &Arena<String>) -> Vec<&str> {
        arena.values().map(String::as_str).collect()
    }

    /// Every handle resolves to its own value, and agrees with the position
    /// table in both directions.
    fn assert_consistent(arena: &Arena<String>) {
        for (position, (id, value)) in arena.iter().enumerate() {
            assert_eq!(arena.position(id), Some(position), "position disagrees");
            assert_eq!(arena.id_at(position), Some(id), "id_at disagrees");
            assert_eq!(arena.get(id), Some(value), "handle resolves elsewhere");
        }
        assert_eq!(arena.iter().count(), arena.len());
    }

    #[test]
    fn a_removed_handle_never_resolves_again() {
        let mut arena = arena_of(&["a", "b", "c"]);
        let b = arena.id_at(1).unwrap();

        assert_eq!(arena.remove(b).as_deref(), Some("b"));
        assert!(!arena.contains(b));
        assert_eq!(arena.get(b), None);
        assert_eq!(arena.position(b), None);

        let reused = arena.push("d".to_owned());
        assert_eq!(
            reused.index, b.index,
            "the test needs the slot to be reused"
        );
        assert_ne!(reused, b);
        assert_eq!(arena.get(reused).map(String::as_str), Some("d"));
        assert_eq!(arena.get(b), None);
        assert!(!arena.contains(b));
        assert_consistent(&arena);
    }

    #[test]
    fn removing_does_not_disturb_other_handles() {
        let mut arena = arena_of(&["a", "b", "c", "d"]);
        let ids: Vec<_> = arena.iter().map(|(id, _)| id).collect();

        let _removed = arena.remove(ids[1]);

        assert_eq!(order(&arena), ["a", "c", "d"]);
        assert_eq!(arena.get(ids[0]).unwrap(), "a");
        assert_eq!(arena.get(ids[2]).unwrap(), "c");
        assert_eq!(arena.get(ids[3]).unwrap(), "d");
        assert_eq!(arena.position(ids[3]), Some(2));
        assert_consistent(&arena);
    }

    #[test]
    fn inserting_shifts_positions_and_keeps_handles() {
        let mut arena = arena_of(&["a", "c"]);
        let c = arena.id_at(1).unwrap();

        let b = arena.insert(1, "b".to_owned());

        assert_eq!(order(&arena), ["a", "b", "c"]);
        assert_eq!(arena.position(b), Some(1));
        assert_eq!(arena.position(c), Some(2));
        assert_consistent(&arena);

        let front = arena.insert(0, "z".to_owned());
        assert_eq!(order(&arena), ["z", "a", "b", "c"]);
        assert_eq!(arena.position(front), Some(0));
        assert_eq!(arena.position(c), Some(3));
        assert_consistent(&arena);
    }

    #[test]
    fn swapping_exchanges_positions() {
        let mut arena = arena_of(&["a", "b", "c"]);
        let a = arena.id_at(0).unwrap();
        let c = arena.id_at(2).unwrap();

        arena.swap(0, 2);

        assert_eq!(order(&arena), ["c", "b", "a"]);
        assert_eq!(arena.position(a), Some(2));
        assert_eq!(arena.position(c), Some(0));
        assert_consistent(&arena);
    }

    #[test]
    fn sorting_reorders_values_without_invalidating_handles() {
        let mut arena = arena_of(&["c", "a", "d", "b"]);
        let ids: Vec<_> = arena.iter().map(|(id, _)| id).collect();

        arena.sort_by_key(Clone::clone);

        assert_eq!(order(&arena), ["a", "b", "c", "d"]);
        assert_eq!(arena.get(ids[0]).unwrap(), "c");
        assert_eq!(arena.get(ids[1]).unwrap(), "a");
        assert_eq!(arena.get(ids[2]).unwrap(), "d");
        assert_eq!(arena.get(ids[3]).unwrap(), "b");
        assert_eq!(arena.position(ids[0]), Some(2));
        assert_eq!(arena.position(ids[1]), Some(0));
        assert_consistent(&arena);
    }

    #[test]
    fn sorting_is_stable() {
        let mut arena = arena_of(&["b1", "a1", "b2", "a2"]);
        let ids: Vec<_> = arena.iter().map(|(id, _)| id).collect();

        arena.sort_by_key(|value| value.as_bytes()[0]);

        assert_eq!(order(&arena), ["a1", "a2", "b1", "b2"]);
        assert_eq!(arena.position(ids[0]), Some(2));
        assert_eq!(arena.position(ids[2]), Some(3));
    }

    #[test]
    fn free_slots_are_reused_without_growing_the_table() {
        let mut arena = arena_of(&["a", "b", "c"]);
        let ids: Vec<_> = arena.iter().map(|(id, _)| id).collect();

        let _removed = arena.remove(ids[0]);
        let _removed = arena.remove(ids[2]);
        assert_eq!(order(&arena), ["b"]);

        let d = arena.push("d".to_owned());
        let e = arena.push("e".to_owned());

        assert_eq!(order(&arena), ["b", "d", "e"]);
        assert_eq!(arena.slots.len(), 3, "no slot should have been added");
        assert!(arena.free.is_empty());
        assert_ne!(d, ids[0]);
        assert_ne!(d, ids[2]);
        assert_ne!(e, ids[0]);
        assert_ne!(e, ids[2]);
        assert_consistent(&arena);
    }

    #[test]
    fn mutation_reaches_the_value_behind_the_handle() {
        let mut arena = arena_of(&["a", "b"]);
        let b = arena.id_at(1).unwrap();

        arena.get_mut(b).unwrap().push('!');
        arena.sort_by_key(Clone::clone);

        assert_eq!(arena.get(b).unwrap(), "b!");
        assert_eq!(order(&arena), ["a", "b!"]);
    }

    #[test]
    fn an_empty_arena_resolves_nothing() {
        let mut arena: Arena<String> = Arena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.id_at(0), None);
        assert_eq!(arena.get_at(0), None);
        assert_eq!(arena.remove_at(0), None);

        let only = arena.push("a".to_owned());
        assert_eq!(arena.remove(only).as_deref(), Some("a"));
        assert!(arena.is_empty());
        assert_eq!(arena.get(only), None);
    }
}
