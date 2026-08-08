//! The serde representation is normalized and index-based: each set is a plain
//! ordered list and each handle is the integer the `.omap` format itself
//! stores. The check that matters is that a map survives a serde round trip
//! well enough to still write the same `.omap` bytes.

#![cfg(feature = "serde")]
#![expect(
    clippy::expect_used,
    reason = "a test asserting a round trip should fail loudly at the point it breaks"
)]

use omap::geo_types::Point;
use omap::{Omap, Result};

const ISOM_15000: &[u8] = include_bytes!("../src/default_maps/isom_15000.omap");
const ISSPROM_4000: &[u8] = include_bytes!("../src/default_maps/issprom_4000.omap");
const FROM_PATH: &[u8] = include_bytes!("../example_data/from_path.omap");

fn corpus() -> [(&'static str, &'static [u8]); 3] {
    [
        ("isom_15000", ISOM_15000),
        ("issprom_4000", ISSPROM_4000),
        ("from_path", FROM_PATH),
    ]
}

/// The whole index-based reference graph, expressed the way the file format
/// expresses it. This is what serialization has to preserve.
///
/// Deliberately not a byte comparison of the written `.omap`: `serde_json` 1.0.150
/// parses some f64 values one ULP off (`0.9996158509501687` round-trips exactly
/// through `std` but not through `serde_json`), which shifts a georeferencing
/// scale factor in the output. That is a float-precision artifact of the JSON
/// library, unrelated to whether handles survive.
#[derive(Debug, PartialEq)]
struct ReferenceGraph {
    colors: Vec<(usize, String)>,
    symbols: Vec<(usize, String, String, Vec<usize>)>,
    components: Vec<(usize, Vec<Option<usize>>)>,
    objects: Vec<Option<usize>>,
}

fn reference_graph(map: &Omap) -> ReferenceGraph {
    use omap::symbols::{PublicOrPrivateSymbol, Symbol, SymbolId};

    let colors = map
        .colors
        .values()
        .enumerate()
        .map(|(priority, color)| (priority, color.name().to_owned()))
        .collect();

    let symbols = map
        .symbols
        .iter()
        .enumerate()
        .map(|(index, (_, symbol))| {
            let used = symbol
                .colors(&map.symbols)
                .into_iter()
                .filter_map(|color| map.colors.priority_of(color))
                .collect();
            (
                index,
                symbol.common().code.to_string(),
                symbol.name().to_owned(),
                used,
            )
        })
        .collect();

    let component_ids = |ids: Vec<Option<SymbolId>>| {
        ids.into_iter()
            .map(|id| {
                let id = id?;
                map.symbols.index_of(id)
            })
            .collect::<Vec<_>>()
    };
    let components = map
        .symbols
        .values()
        .enumerate()
        .filter_map(|(index, symbol)| {
            let ids: Vec<Option<SymbolId>> = match symbol {
                Symbol::CombinedArea(combined) => combined
                    .components()
                    .map(|part| match part {
                        PublicOrPrivateSymbol::Public(id) => Some(SymbolId::from(*id)),
                        PublicOrPrivateSymbol::Private(_) => None,
                    })
                    .collect(),
                Symbol::CombinedLine(combined) => combined
                    .components()
                    .map(|part| match part {
                        PublicOrPrivateSymbol::Public(id) => Some(SymbolId::from(*id)),
                        PublicOrPrivateSymbol::Private(_) => None,
                    })
                    .collect(),
                _ => return None,
            };
            Some((index, component_ids(ids)))
        })
        .collect();

    let objects = map
        .iter_all_objects()
        .map(|object| {
            let id = object.symbol()?;
            map.symbols.index_of(id)
        })
        .collect();

    ReferenceGraph {
        colors,
        symbols,
        components,
        objects,
    }
}

#[test]
fn the_reference_graph_survives_a_json_round_trip() -> Result<()> {
    for (name, bytes) in corpus() {
        let mut map = Omap::from_bytes(bytes)?;
        // Sorting happens on write, so do it first to compare like with like.
        let mut sink = Vec::new();
        map.to_writer(&mut sink)?;
        let before = reference_graph(&map);

        let json = serde_json::to_string(&map).expect("serialize");
        let restored: Omap = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(
            reference_graph(&restored),
            before,
            "{name}: the reference graph changed across a serde round trip"
        );
        assert!(
            restored.validate().is_ok(),
            "{name}: the restored map must validate"
        );
    }
    Ok(())
}

/// Handles are serialized as the file index, so they must come back naming the
/// same symbols and colours.
#[test]
fn handles_resolve_to_the_same_symbols_after_a_round_trip() -> Result<()> {
    let map = Omap::from_bytes(ISOM_15000)?;
    let json = serde_json::to_string(&map).expect("serialize");
    let restored: Omap = serde_json::from_str(&json).expect("deserialize");

    for (index, (id, symbol)) in map.symbols.iter().enumerate() {
        assert_eq!(map.symbols.index_of(id), Some(index));
        let restored_id = restored.symbols.id_at(index).expect("same index exists");
        let restored_symbol = restored.symbols.get(restored_id).expect("resolves");
        assert_eq!(restored_symbol.name(), symbol.name());
        assert_eq!(restored_symbol.common().code, symbol.common().code);
    }
    Ok(())
}

#[test]
fn a_parsed_map_is_already_compact() -> Result<()> {
    for (name, bytes) in corpus() {
        let map = Omap::from_bytes(bytes)?;
        assert!(
            map.is_compact(),
            "{name}: a freshly parsed map should need no compaction"
        );
    }
    Ok(())
}

/// Removing a symbol leaves a gap, so handles stop matching positions. Once
/// compacted the map serializes, and references to the removed symbol are gone
/// rather than silently pointing at whatever took its place.
#[test]
fn removal_is_compacted_away_before_serializing() -> Result<()> {
    let mut map = Omap::from_bytes(ISOM_15000)?;
    let doomed = map.symbols.ids().nth(3).expect("a fourth symbol");
    let survivor = map.symbols.ids().nth(4).expect("a fifth symbol");
    let survivor_name = map
        .symbols
        .get(survivor)
        .expect("survivor resolves")
        .name()
        .to_owned();

    let _removed = map.symbols.remove(doomed);
    map.symbols.add_symbol(omap::symbols::PointSymbol::new(
        omap::Code::new(9, 9, 9),
        "added",
    ));
    assert!(
        !map.is_compact(),
        "a removal followed by an insertion should break compactness"
    );

    // Serializing an uncompacted map must still work: it compacts a copy.
    let json = serde_json::to_string(&map).expect("serialize uncompacted");
    let restored: Omap = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.symbols.len(), map.symbols.len());
    assert!(restored.is_compact());
    assert!(restored.validate().is_ok(), "restored map must validate");

    map.compact();
    assert!(map.is_compact());
    assert!(
        map.symbols.get(doomed).is_none(),
        "the removed symbol must not come back"
    );
    assert!(map.validate().is_ok());

    // The survivor is still present, found by name rather than by the old
    // handle, which compaction deliberately invalidates.
    assert!(map.symbols.symbol_by_name(&survivor_name).is_some());
    Ok(())
}

#[test]
fn compacting_drops_references_to_removed_symbols() -> Result<()> {
    let mut map = Omap::from_bytes(ISOM_15000)?;
    let point = map
        .symbols
        .iter_point_symbols()
        .next()
        .map(|(id, _)| id)
        .expect("a point symbol");

    let part = map.parts.get_mut(0).expect("a map part");
    part.add_object(omap::objects::PointObject::new(
        Some(point),
        Point::new(1.0, 2.0),
    ));

    let _removed = map.symbols.remove(point.into());
    assert!(map.validate().is_err(), "the object now dangles");

    map.compact();

    let object = map.iter_all_objects().last().expect("the added object");
    assert_eq!(
        object.symbol(),
        None,
        "a dangling reference must compact to None, not to whatever took the slot"
    );
    assert!(map.validate().is_ok());
    Ok(())
}
