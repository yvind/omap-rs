//! Every lookup hands back a `SymbolRef`: the symbol for reading, the handle
//! for storing, and narrowing through `TryFrom` or the `as_*` helpers.

use omap::{
    Code, Omap, Result,
    colors::{ColorId, ColorKind, ColorRef, MixedColorId, SpotColorId},
    symbols::{
        LinePathSymbolId, LineSymbolId, PathSymbolId, PointSymbolId, Symbol, SymbolId, SymbolKind,
    },
};

fn erosion_gully() -> Code {
    Code::new(107, 0, 0)
}

fn isom() -> Result<Omap> {
    Omap::from_bytes(include_bytes!("../src/default_maps/isom_15000.omap"))
}

#[test]
fn a_lookup_reads_the_symbol_and_yields_the_handle() -> Result<()> {
    let map = isom()?;
    let found = map
        .symbols
        .find_by_code(erosion_gully())
        .ok_or(omap::Error::ObjectError)?;

    assert_eq!(found.code(), erosion_gully());
    assert_eq!(
        map.symbols
            .get(found.id())
            .map(|found| found.symbol().name()),
        Some(found.name())
    );
    Ok(())
}

#[test]
fn narrowing_composes_with_every_lookup_key() -> Result<()> {
    let map = isom()?;

    let by_code = map
        .symbols
        .find_by_code(erosion_gully())
        .and_then(|symbol| symbol.as_line_path())
        .ok_or(omap::Error::ObjectError)?;

    let name = map
        .symbols
        .find_by_code(erosion_gully())
        .ok_or(omap::Error::ObjectError)?
        .name()
        .to_owned();
    let by_name = map
        .symbols
        .find_by_name(&name)
        .and_then(|symbol| symbol.as_line_path())
        .ok_or(omap::Error::ObjectError)?;

    let index = map
        .symbols
        .index_of(by_code.into())
        .ok_or(omap::Error::ObjectError)?;
    let at_index = map
        .symbols
        .find_at(index)
        .and_then(|symbol| symbol.as_line_path())
        .ok_or(omap::Error::ObjectError)?;

    assert_eq!(by_code, by_name);
    assert_eq!(by_code, at_index);
    Ok(())
}

#[test]
fn narrowing_to_the_wrong_kind_is_none() -> Result<()> {
    let map = isom()?;
    let found = map
        .symbols
        .find_by_code(erosion_gully())
        .ok_or(omap::Error::ObjectError)?;

    assert!(matches!(found.symbol(), Symbol::Line(_)));
    assert!(found.as_line().is_some());
    assert!(found.as_line_path().is_some());
    assert!(found.as_path().is_some());
    assert!(found.as_point().is_none());
    assert!(found.as_text().is_none());
    assert!(found.as_area().is_none());
    assert!(found.as_combined_line().is_none());
    Ok(())
}

#[test]
fn narrowing_uses_try_from() -> Result<()> {
    let map = isom()?;
    let found = map
        .symbols
        .find_by_code(erosion_gully())
        .ok_or(omap::Error::ObjectError)?;

    let line = LineSymbolId::try_from(found)?;
    let line_path = LinePathSymbolId::try_from(found)?;
    let path = PathSymbolId::try_from(found)?;
    let untyped = SymbolId::try_from(found)?;

    assert_eq!(SymbolId::from(line), found.id());
    assert_eq!(SymbolId::from(line_path), found.id());
    assert_eq!(SymbolId::from(path), found.id());
    assert_eq!(untyped, found.id());

    assert!(matches!(
        PointSymbolId::try_from(found),
        Err(omap::Error::SymbolKindMismatch {
            expected: &[SymbolKind::Point],
            found: SymbolKind::Line,
        })
    ));
    Ok(())
}

#[test]
fn narrowing_from_a_bare_handle_goes_through_get() -> Result<()> {
    let map = isom()?;
    let id = map
        .symbols
        .find_by_code(erosion_gully())
        .ok_or(omap::Error::ObjectError)?
        .id();

    let narrowed = map
        .symbols
        .get(id)
        .and_then(|symbol| symbol.as_line_path())
        .ok_or(omap::Error::ObjectError)?;

    assert_eq!(
        map.symbols.index_of(narrowed.into()),
        map.symbols.index_of(id)
    );
    Ok(())
}

#[test]
fn a_missing_symbol_is_none() -> Result<()> {
    let map = isom()?;

    assert!(map.symbols.find_by_code(Code::new(9999, 0, 0)).is_none());
    assert!(map.symbols.find_by_name("no such symbol").is_none());
    assert!(map.symbols.find_at(map.symbols.len()).is_none());
    Ok(())
}

/// The handle is `Copy` and outlives the borrow the lookup took, so the set can
/// be mutated once it has been extracted.
#[test]
fn the_handle_outlives_the_borrow() -> Result<()> {
    let mut map = isom()?;
    let id = map
        .symbols
        .find_by_code(erosion_gully())
        .and_then(|symbol| symbol.as_line_path())
        .ok_or(omap::Error::ObjectError)?;

    map.symbols.sort();

    assert_eq!(
        map.symbols.get(id.into()).map(|symbol| symbol.code()),
        Some(erosion_gully())
    );
    Ok(())
}

#[test]
fn a_color_lookup_yields_the_handle_and_the_color() -> Result<()> {
    let map = isom()?;
    let name = map
        .colors
        .values()
        .next()
        .ok_or(omap::Error::ObjectError)?
        .name()
        .to_owned();

    let found = map
        .colors
        .find_by_name(&name)
        .ok_or(omap::Error::ObjectError)?;

    assert_eq!(found.name(), name);
    assert_eq!(
        map.colors.get(found.id()).map(ColorRef::id),
        Some(found.id())
    );
    assert!(found.as_spot().is_some() || found.as_mixed().is_some());
    Ok(())
}

#[test]
fn color_narrowing_uses_try_from() -> Result<()> {
    let map = isom()?;
    let spot = map
        .colors
        .iter()
        .find(|color| color.kind() == ColorKind::Spot)
        .ok_or(omap::Error::ColorError)?;
    let mixed = map
        .colors
        .iter()
        .find(|color| color.kind() == ColorKind::Mixed)
        .ok_or(omap::Error::ColorError)?;

    let spot_id = SpotColorId::try_from(spot)?;
    let mixed_id = MixedColorId::try_from(mixed)?;
    assert_eq!(ColorId::from(spot_id), spot.id());
    assert_eq!(ColorId::from(mixed_id), mixed.id());
    assert_eq!(ColorId::try_from(spot)?, spot.id());

    assert!(matches!(
        MixedColorId::try_from(spot),
        Err(omap::Error::ColorKindMismatch {
            expected: &[ColorKind::Mixed],
            found: ColorKind::Spot,
        })
    ));
    Ok(())
}
