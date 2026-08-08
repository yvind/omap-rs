//! Test that read - write - read does not change the map

use omap::{Omap, Result};

const ISOM_15000: &[u8] = include_bytes!("../src/default_maps/isom_15000.omap");
const ISOM_10000: &[u8] = include_bytes!("../src/default_maps/isom_10000.omap");
const ISSPROM_4000: &[u8] = include_bytes!("../src/default_maps/issprom_4000.omap");
const FROM_PATH: &[u8] = include_bytes!("../example_data/from_path.omap");

fn corpus() -> [(&'static str, &'static [u8]); 4] {
    [
        ("isom_15000", ISOM_15000),
        ("isom_10000", ISOM_10000),
        ("issprom_4000", ISSPROM_4000),
        ("from_path", FROM_PATH),
    ]
}

fn round_trip(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut map = Omap::from_bytes(bytes)?;
    let mut out = Vec::new();
    map.to_writer(&mut out)?;
    Ok(out)
}

/// The interior of every `<name ...>` start tag, excluding the angle brackets.
fn start_tags<'a>(xml: &'a str, name: &str) -> Vec<&'a str> {
    let needle = format!("<{name}");
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(pos) = xml[offset..].find(&needle) {
        let start = offset + pos + needle.len();
        offset = start;
        if !matches!(xml[start..].chars().next(), Some(' ' | '>' | '/')) {
            continue;
        }
        if let Some(end) = xml[start..].find('>') {
            out.push(&xml[start..start + end]);
        }
    }
    out
}

/// The value of `name="..."` in a start tag, matching whole attribute names
/// only so that `symbol` does not match inside `is_helper_symbol`.
fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let mut offset = 0;
    while let Some(pos) = tag[offset..].find(&needle) {
        let at = offset + pos;
        offset = at + needle.len();
        let preceded_by_boundary = at == 0 || tag.as_bytes()[at - 1] == b' ';
        if !preceded_by_boundary {
            continue;
        }
        let rest = &tag[offset..];
        return rest.find('"').map(|end| rest[..end].to_owned());
    }
    None
}

fn sorted<T: Ord>(mut values: Vec<T>) -> Vec<T> {
    values.sort();
    values
}

/// The `<name> … </name>` region, so that the `undo` and `redo` histories --
/// which this crate deliberately discards -- are not mistaken for map content.
fn section<'a>(xml: &'a str, name: &str) -> &'a str {
    let Some(open) = xml
        .find(&format!("<{name} "))
        .or_else(|| xml.find(&format!("<{name}>")))
    else {
        return "";
    };
    let close = xml[open..]
        .find(&format!("</{name}>"))
        .map_or(xml.len(), |end| open + end);
    &xml[open..close]
}

/// `Code` is three integers, so `603.0` and `603` are the same code and the
/// writer emits the shorter form. Compare on that normal form.
fn normalized_code(code: &str) -> String {
    let mut parts: Vec<&str> = code.split('.').collect();
    while parts.len() > 1 && parts.last() == Some(&"0") {
        parts.pop();
    }
    parts.join(".")
}

fn only_differences<T: Clone + Ord>(left: &[T], right: &[T]) -> (Vec<T>, Vec<T>) {
    let missing = left
        .iter()
        .filter(|v| !right.contains(v))
        .cloned()
        .collect();
    let added = right
        .iter()
        .filter(|v| !left.contains(v))
        .cloned()
        .collect();
    (missing, added)
}

/// Symbol file index -> symbol code, for the top-level symbols of one document.
/// Only top-level symbols carry an `id` attribute; sub-symbols and private
/// combined-symbol parts do not.
fn symbol_codes(xml: &str) -> std::collections::HashMap<String, String> {
    start_tags(section(xml, "symbols"), "symbol")
        .into_iter()
        .filter_map(|tag| {
            Some((
                attr(tag, "id")?,
                normalized_code(&attr(tag, "code").unwrap_or_default()),
            ))
        })
        .collect()
}

/// Every symbol reference in the document, resolved to the referenced symbol's
/// code so the comparison survives the write path's sort-by-code reordering.
/// A reference that resolves to nothing is reported verbatim (`-1`, typically).
fn references(xml: &str, scope: &str, element: &str) -> Vec<String> {
    let codes = symbol_codes(xml);
    start_tags(section(xml, scope), element)
        .into_iter()
        .filter_map(|tag| attr(tag, "symbol"))
        .map(|id| codes.get(&id).cloned().unwrap_or(id))
        .collect()
}

#[test]
fn writing_is_a_fixed_point() -> Result<()> {
    for (name, bytes) in corpus() {
        let first = round_trip(bytes)?;
        let second = round_trip(&first)?;
        assert_eq!(
            first.len(),
            second.len(),
            "{name}: second write changed length, so a reference did not survive the round trip"
        );
        assert!(
            first == second,
            "{name}: writing is not idempotent from the second generation"
        );
    }
    Ok(())
}

#[test]
fn colors_survive_the_round_trip() -> Result<()> {
    for (name, bytes) in corpus() {
        let original = String::from_utf8_lossy(bytes).into_owned();
        let written = round_trip(bytes)?;
        let written = String::from_utf8_lossy(&written).into_owned();

        let original_colors = start_tags(&original, "color");
        let written_colors = start_tags(&written, "color");
        assert_eq!(
            original_colors.len(),
            written_colors.len(),
            "{name}: colour count changed"
        );

        let names = |tags: &[&str]| {
            sorted(
                tags.iter()
                    .filter_map(|tag| Some((attr(tag, "priority")?, attr(tag, "name")?)))
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(
            names(&original_colors),
            names(&written_colors),
            "{name}: colour names or priorities changed"
        );

        let components = |xml: &str| {
            sorted(
                start_tags(xml, "component")
                    .into_iter()
                    .filter_map(|tag| attr(tag, "spotcolor"))
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(
            components(&original),
            components(&written),
            "{name}: mixed-colour spot references changed"
        );
    }
    Ok(())
}

#[test]
fn symbols_survive_the_round_trip() -> Result<()> {
    for (name, bytes) in corpus() {
        let original = String::from_utf8_lossy(bytes).into_owned();
        let written = round_trip(bytes)?;
        let written = String::from_utf8_lossy(&written).into_owned();

        let identity = |xml: &str| {
            sorted(
                start_tags(section(xml, "symbols"), "symbol")
                    .into_iter()
                    .filter(|tag| attr(tag, "id").is_some())
                    .filter_map(|tag| {
                        Some((
                            attr(tag, "type")?,
                            normalized_code(&attr(tag, "code")?),
                            attr(tag, "name").unwrap_or_default(),
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let (missing, added) = only_differences(&identity(&original), &identity(&written));
        assert!(
            missing.is_empty() && added.is_empty(),
            "{name}: the set of top-level symbols changed\n  lost: {missing:?}\n  gained: {added:?}"
        );

        let live = |xml: &str| {
            sorted(
                references(xml, "symbols", "part")
                    .into_iter()
                    .filter(|reference| reference != "-1")
                    .collect::<Vec<_>>(),
            )
        };
        let (missing, added) = only_differences(&live(&original), &live(&written));
        assert!(
            missing.is_empty() && added.is_empty(),
            "{name}: combined-symbol component references changed\n  lost: {missing:?}\n  gained: {added:?}"
        );
    }
    Ok(())
}

#[test]
fn objects_survive_the_round_trip() -> Result<()> {
    for (name, bytes) in corpus() {
        let original = String::from_utf8_lossy(bytes).into_owned();
        let written = round_trip(bytes)?;
        let written = String::from_utf8_lossy(&written).into_owned();

        assert_eq!(
            sorted(references(&original, "parts", "object")),
            sorted(references(&written, "parts", "object")),
            "{name}: the symbols referenced by objects changed"
        );
    }
    Ok(())
}

#[test]
fn no_reference_degrades_to_the_unknown_sentinel() -> Result<()> {
    for (name, bytes) in corpus() {
        let original = String::from_utf8_lossy(bytes).into_owned();
        let written = round_trip(bytes)?;
        let written = String::from_utf8_lossy(&written).into_owned();

        let dangling = |xml: &str, scope: &str, element: &str| {
            start_tags(section(xml, scope), element)
                .into_iter()
                .filter(|tag| attr(tag, "symbol").as_deref() == Some("-1"))
                .count()
        };
        for (scope, element) in [("parts", "object"), ("symbols", "part")] {
            assert!(
                dangling(&written, scope, element) <= dangling(&original, scope, element),
                "{name}: writing turned a live <{element}> symbol reference into -1"
            );
        }
    }
    Ok(())
}
