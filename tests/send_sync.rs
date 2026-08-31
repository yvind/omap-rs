//! Make sure that the `Omap` type and its components are `Send`, `Sync` and `Clone`.

use omap::{Omap, Result};

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}
fn assert_clone<T: Clone>() {}

#[test]
fn a_parsed_map_is_send_sync_and_clone() {
    assert_send::<Omap>();
    assert_sync::<Omap>();
    assert_clone::<Omap>();

    assert_send::<omap::symbols::SymbolSet>();
    assert_sync::<omap::symbols::SymbolSet>();
    assert_send::<omap::colors::ColorSet>();
    assert_sync::<omap::colors::ColorSet>();
    assert_send::<omap::objects::MapObject>();
    assert_sync::<omap::objects::MapObject>();
}

#[test]
fn handles_are_copy_and_hashable() {
    fn assert_handle<T: Copy + Eq + std::hash::Hash + Send + Sync>() {}

    assert_handle::<omap::symbols::SymbolId>();
    assert_handle::<omap::symbols::PointSymbolId>();
    assert_handle::<omap::symbols::LinePathSymbolId>();
    assert_handle::<omap::symbols::AreaPathSymbolId>();
    assert_handle::<omap::symbols::PathSymbolId>();
    assert_handle::<omap::colors::ColorId>();
    assert_handle::<omap::colors::SpotColorId>();
}

#[test]
fn a_map_can_be_moved_to_a_worker_thread() -> Result<()> {
    let map = Omap::from_bytes(include_bytes!("../src/default_maps/isom_15000.omap"))?;
    let symbols = map.symbols.len();

    let counted = std::thread::spawn(move || map.symbols.len())
        .join()
        .map_err(|_panic| omap::Error::ObjectError)?;

    assert_eq!(counted, symbols);
    Ok(())
}

/// Cloning preserves slot indices and generations, so handles taken from the
/// original still resolve against the clone.
#[test]
fn handles_survive_a_clone() -> Result<()> {
    let map = Omap::from_bytes(include_bytes!("../src/default_maps/isom_15000.omap"))?;
    let id = map.symbols.ids().next().ok_or(omap::Error::ObjectError)?;
    let name = map
        .symbols
        .get(id)
        .ok_or(omap::Error::ObjectError)?
        .name()
        .to_owned();

    let clone = map.clone();
    assert_eq!(
        clone.symbols.get(id).map(|symbol| symbol.name()),
        Some(name.as_str())
    );
    assert_eq!(clone.symbols.index_of(id), map.symbols.index_of(id));
    Ok(())
}

/// A handle from one map must not silently resolve against another.
#[test]
fn a_removed_symbol_stops_resolving() -> Result<()> {
    let mut map = Omap::from_bytes(include_bytes!("../src/default_maps/isom_15000.omap"))?;
    let id = map.symbols.ids().next().ok_or(omap::Error::ObjectError)?;
    assert!(map.symbols.contains(id));

    let _removed = map.symbols.remove(id);

    assert!(!map.symbols.contains(id));
    assert!(map.symbols.get(id).is_none());
    assert!(map.symbols.index_of(id).is_none());
    Ok(())
}
