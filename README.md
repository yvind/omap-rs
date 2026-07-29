# Omap-rs
[![crates.io version](https://img.shields.io/crates/v/omap.svg)](https://crates.io/crates/omap)
[![docs.rs docs](https://docs.rs/omap/badge.svg)](https://docs.rs/omap)  

A library for working with OpenOrienteering Mapper's .omap files.

For writing new files you can either start with a completely empty map `Omap::new` or use one of the provided templates `Omap::default_15_000`, `Omap::default_10_000` or `Omap::default_4_000`.
Or you can start from an already existing file with `Omap::from_path`.

## Geo-referencing
With the `geo_ref`-feature automatic geo-referencing with magnetic north and scale factor calculation is enabled and done with the `omap::GeoRef::initialize` function. \
It is not enabled by default because of the extra dependencies needed (Proj4rs for coordinate projections, WMM for magnetic north calcualtion and Chrono for time as the magnetic north changes over time). Without this feature the georeferencing must be done by hand.

**NB!** if you change any field (or the entire thing) in the map's `geo_referencing`-field then all the map objects projected/geographic positions will change as their coordinates are given in mm-of-paper and remain untouched.
The best practice is to set the map's geo referencing before adding any objects.

`omap::geo_referencing::MapTransform` provides functions for going back and forth between mm-of-paper and projected coordinates given by map's georeferencing. And is obtained with calling `get_transform` on the map's `geo_referencing`-field.

`MapTransform::affine_between` can be used to keep objects and non-georeferenced templates at the same projected positions after changing map reference point, scale, or rotation within the same CRS representation. It deliberately rejects transforms whose CRS fingerprints differ, for example different EPSG codes.
This is a conservative lightweight guard; `omap-rs` does not normalize equivalent CRS definitions or transform coordinates between different projections.

## Dash-points and beziers

Line and area objects expose their exact mixed straight/cubic geometry through
`bezier_geometry()`. Its per-vertex metadata includes forced dash points,
including both endpoints of an open path. The ordinary `get_geometry(error)`
API lazily provides and caches a flattened `LineString` or `Polygon`.

Objects that are not edited are written back using their original coordinates
and flags. Added or edited line and area objects can be fitted to Bézier curves
on write by setting `bezier_write_error`.

## Example

```Rust
fn main() {
    let proj_center = Coord {
        x: 463_575.5,
        y: 6_833_849.6,
    };
    let map_center_elevation_meters = 2_469.;
    let crs_epsg_code = 25832;

    // feature "geo_ref" is activated
    let mut map = Omap::default_15_000_geo_referenced(
        proj_center,
        CrsType::Epsg(crs_epsg_code),
        map_center_elevation_meters,
    ).unwrap();

    // Iterate through the colors
    for color in map.colors.iter() {
        match color {
            // Colors are split between `SpotColor` which defines new colors
            Color::SpotColor(ref_cell) => {
                let b = ref_cell.try_borrow().unwrap();
                println!("{} with spot name {}", b.color_name, b.spotcolor_name);
            }
            // Or `MixedColor` which are made up of weighted `SpotColor`-components
            Color::MixedColor(ref_cell) => {
                println!("{}", ref_cell.try_borrow().unwrap().color_name);
            }
        }
    }

    // The Symbol set holds `Rc`s (owning pointers) of the symbols (which again hold weak pointers of the colors)
    // The objects hold weak pointers of the symbol
    let erosion_gully = map
        .symbols
        .get_symbol_by_code(Code::new(107, 0, 0))
        .unwrap()
        .downgrade();

    // O-mapper makes no difference between line objects and area objects, but we do as we repesent them with LineString and Polygon
    let mut ls = LineObject::new(
        WeakLinePathSymbol::try_from(erosion_gully).unwrap(),
        // geometry coordinates are always in mm of paper
        LineString::new(vec![Coord { x: 0., y: 0. }, Coord { x: 200., y: 100. }]),
    );
    // Fit this LineString to cubic Béziers with at most 0.2 mm error on write.
    ls.bezier_write_error = Some(omap::NonNegativeF64::clamped_from(0.2));
    // Add some tags to the object
    ls.tags.insert("Some Key".to_string(), "My value".to_string());

    // A map can have multiple parts let's add the object to the first one
    map.parts.get_map_part_by_index_mut(0).unwrap().add_object(ls);

    // O-mapper makes no difference between combined line symbols and combined area symbols, but we do
    // This will debug-print a `CombinedLineSymbol`
    if let Some(s) = map.symbols.get_symbol_by_name("Railway, Olive background") {
        dbg!(s);
    }

    map.write_to_file("./test_write.omap")
}
```
