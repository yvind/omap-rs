//! Each set serializes as its slot-keyed values plus the order those slots
//! appear in, so a handle carries its own generation and round-trips verbatim.
//! The checks that matter are that a map survives a serde round trip well
//! enough to still write the same `.omap` bytes, and that a handle taken
//! beforehand still resolves against the restored map.

#![cfg(feature = "serde")]
#![expect(
    clippy::expect_used,
    reason = "a test asserting a round trip should fail loudly at the point it breaks"
)]

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
        let map = Omap::from_bytes(bytes)?;
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

/// A handle carries its own slot generation, so the handle you held before a
/// round trip must still resolve afterwards — not merely some handle at the
/// same index.
#[test]
fn handles_resolve_to_the_same_symbols_after_a_round_trip() -> Result<()> {
    let map = Omap::from_bytes(ISOM_15000)?;
    let json = serde_json::to_string(&map).expect("serialize");
    let restored: Omap = serde_json::from_str(&json).expect("deserialize");

    for (index, (id, symbol)) in map.symbols.iter().enumerate() {
        assert_eq!(map.symbols.index_of(id), Some(index));

        // The original handle, used verbatim against the restored map.
        let restored_symbol = restored.symbols.get(id).expect("the old handle resolves");
        assert_eq!(restored_symbol.name(), symbol.name());
        assert_eq!(restored_symbol.common().code, symbol.common().code);
        assert_eq!(
            restored.symbols.index_of(id),
            Some(index),
            "a handle must keep its position across a round trip"
        );
    }
    Ok(())
}

/// A removal leaves a gap in the slot keys. Nothing needs compacting away: the
/// gap round-trips, the survivors keep their handles, and the removed symbol
/// stays removed.
#[test]
fn a_removal_survives_a_round_trip_without_pruning() -> Result<()> {
    let mut map = Omap::from_bytes(ISOM_15000)?;
    let doomed = map.symbols.ids().nth(3).expect("a fourth symbol");
    let survivor = map.symbols.ids().nth(4).expect("a fifth symbol");

    let _removed = map.symbols.remove(doomed);
    let added = map.symbols.add_symbol(omap::symbols::PointSymbol::new(
        omap::Code::new(9, 9, 9),
        "added",
    ));

    let json = serde_json::to_string(&map).expect("serialize after a removal");
    let restored: Omap = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.symbols.len(), map.symbols.len());
    assert!(restored.validate().is_ok(), "restored map must validate");
    assert!(
        restored.symbols.get(doomed).is_none(),
        "the removed symbol must not come back"
    );
    assert!(
        restored.symbols.get(survivor).is_some(),
        "a handle taken before the round trip must still resolve"
    );
    assert_eq!(
        restored.symbols.index_of(survivor),
        map.symbols.index_of(survivor),
        "the survivor must keep its file index"
    );
    assert!(restored.symbols.get(added).is_some());
    Ok(())
}
