//! An ordered arena over a [`slotmap::SlotMap`]. `order` is a permutation of
//! the live keys of `values`.

use slotmap::{Key, SlotMap};

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub(crate) struct Arena<T, K: Key> {
    values: SlotMap<K, T>,
    order: Vec<K>,
}

impl<T, K: Key> Default for Arena<T, K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Debug, K: Key> std::fmt::Debug for Arena<T, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.values()).finish()
    }
}

impl<T, K: Key> Arena<T, K> {
    pub(crate) fn new() -> Self {
        Self {
            values: SlotMap::with_key(),
            order: Vec::new(),
        }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            values: SlotMap::with_capacity_and_key(capacity),
            order: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.order.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub(crate) fn position(&self, id: K) -> Option<usize> {
        self.order.iter().position(|&key| key == id)
    }

    pub(crate) fn contains(&self, id: K) -> bool {
        self.values.contains_key(id)
    }

    pub(crate) fn get(&self, id: K) -> Option<&T> {
        self.values.get(id)
    }

    pub(crate) fn get_mut(&mut self, id: K) -> Option<&mut T> {
        self.values.get_mut(id)
    }

    pub(crate) fn get_at(&self, position: usize) -> Option<&T> {
        self.values.get(*self.order.get(position)?)
    }

    pub(crate) fn get_at_mut(&mut self, position: usize) -> Option<&mut T> {
        let key = *self.order.get(position)?;
        self.values.get_mut(key)
    }

    pub(crate) fn id_at(&self, position: usize) -> Option<K> {
        self.order.get(position).copied()
    }

    pub(crate) fn push(&mut self, value: T) -> K {
        let key = self.values.insert(value);
        self.order.push(key);
        key
    }

    /// # Panics
    ///
    /// Panics if `position > self.len()`.
    pub(crate) fn insert(&mut self, position: usize, value: T) -> K {
        assert!(position <= self.order.len(), "position out of bounds");
        let key = self.values.insert(value);
        self.order.insert(position, key);
        key
    }

    pub(crate) fn remove(&mut self, id: K) -> Option<T> {
        let position = self.position(id)?;
        self.remove_at(position)
    }

    pub(crate) fn remove_at(&mut self, position: usize) -> Option<T> {
        if position >= self.order.len() {
            return None;
        }
        self.values.remove(self.order.remove(position))
    }

    pub(crate) fn swap(&mut self, first: usize, second: usize) {
        self.order.swap(first, second);
    }

    pub(crate) fn sort_by_key<S: Ord, F: Fn(&T) -> S>(&mut self, key: F) {
        let values = &self.values;
        self.order.sort_by_key(|slot| key(&values[*slot]));
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (K, &T)> {
        self.order
            .iter()
            .filter_map(|&key| Some((key, self.values.get(key)?)))
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &T> {
        self.order.iter().filter_map(|&key| self.values.get(key))
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.values.values_mut()
    }
}

impl<T, K: Key> FromIterator<T> for Arena<T, K> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut arena = Self::with_capacity(iter.size_hint().0);
        for value in iter {
            let _key = arena.push(value);
        }
        arena
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>, K: serde::Deserialize<'de>"))]
struct ArenaRepr<T, K: Key> {
    values: SlotMap<K, T>,
    order: Vec<K>,
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>, K: Key + serde::Deserialize<'de>> serde::Deserialize<'de>
    for Arena<T, K>
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let ArenaRepr { values, order } = ArenaRepr::<T, K>::deserialize(deserializer)?;

        if order.len() != values.len() {
            return Err(D::Error::custom("arena order does not cover every value"));
        }
        let mut seen = std::collections::HashSet::with_capacity(order.len());
        for &key in &order {
            if !values.contains_key(key) {
                return Err(D::Error::custom("arena order names a missing value"));
            }
            if !seen.insert(key) {
                return Err(D::Error::custom("arena order names a value twice"));
            }
        }

        Ok(Self { values, order })
    }
}

#[expect(clippy::unwrap_used, reason = "tests assert on known-live handles")]
#[cfg(test)]
mod tests {
    use super::Arena;
    use slotmap::Key as _;

    slotmap::new_key_type! { struct TestKey; }

    type TestArena = Arena<String, TestKey>;

    fn arena_of(values: &[&str]) -> TestArena {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn order(arena: &TestArena) -> Vec<&str> {
        arena.values().map(String::as_str).collect()
    }

    /// The slot a key names, independent of its generation.
    fn slot(key: TestKey) -> u64 {
        key.data().as_ffi() & 0xffff_ffff
    }

    fn assert_consistent(arena: &TestArena) {
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
            slot(reused),
            slot(b),
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
    fn slots_are_reused_without_aliasing() {
        let mut arena = arena_of(&["a", "b", "c"]);
        let ids: Vec<_> = arena.iter().map(|(id, _)| id).collect();

        let _removed = arena.remove(ids[0]);
        let _removed = arena.remove(ids[2]);
        assert_eq!(order(&arena), ["b"]);

        let d = arena.push("d".to_owned());
        let e = arena.push("e".to_owned());

        assert_eq!(order(&arena), ["b", "d", "e"]);

        let freed = [slot(ids[0]), slot(ids[2])];
        assert!(freed.contains(&slot(d)), "a freed slot should be reused");
        assert!(freed.contains(&slot(e)), "a freed slot should be reused");

        // Reusing the slot must not resurrect the old handle.
        assert_ne!(d, ids[0]);
        assert_ne!(d, ids[2]);
        assert_ne!(e, ids[0]);
        assert_ne!(e, ids[2]);
        assert!(!arena.contains(ids[0]));
        assert!(!arena.contains(ids[2]));
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
        let mut arena: TestArena = Arena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.id_at(0), None);
        assert_eq!(arena.get_at(0), None);
        assert_eq!(arena.remove_at(0), None);

        let only = arena.push("a".to_owned());
        assert_eq!(arena.remove(only).as_deref(), Some("a"));
        assert!(arena.is_empty());
        assert_eq!(arena.get(only), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn an_arena_round_trips_through_serde() {
        let mut arena = arena_of(&["c", "a", "d", "b"]);
        let doomed = arena.id_at(2).unwrap();
        let _removed = arena.remove(doomed);
        let added = arena.push("e".to_owned());
        arena.sort_by_key(Clone::clone);

        let before: Vec<(TestKey, String, usize)> = arena
            .iter()
            .map(|(id, value)| (id, value.clone(), arena.position(id).unwrap()))
            .collect();

        let json = serde_json::to_string(&arena).unwrap();
        let restored: TestArena = serde_json::from_str(&json).unwrap();

        assert_consistent(&restored);
        assert_eq!(order(&restored), order(&arena));
        for (id, value, position) in before {
            assert_eq!(restored.get(id).map(String::as_str), Some(value.as_str()));
            assert_eq!(restored.position(id), Some(position));
        }
        assert!(restored.get(doomed).is_none(), "a removed value stays gone");
        assert_eq!(restored.get(added).map(String::as_str), Some("e"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_broken_order_is_rejected() {
        let arena = arena_of(&["a", "b", "c"]);
        let json = serde_json::to_string(&arena).unwrap();
        let repr: serde_json::Value = serde_json::from_str(&json).unwrap();

        let mut short = repr.clone();
        short["order"].as_array_mut().unwrap().pop();
        assert!(
            serde_json::from_value::<TestArena>(short).is_err(),
            "an order that misses a value must be rejected"
        );

        let mut duplicated = repr.clone();
        let first = duplicated["order"][0].clone();
        duplicated["order"].as_array_mut().unwrap()[1] = first;
        assert!(
            serde_json::from_value::<TestArena>(duplicated).is_err(),
            "an order that names a value twice must be rejected"
        );

        let mut missing = repr;
        missing["values"].as_array_mut().unwrap().pop();
        assert!(
            serde_json::from_value::<TestArena>(missing).is_err(),
            "an order naming a missing value must be rejected"
        );
    }
}
