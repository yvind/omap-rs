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

`omap::geo_referencing::Transform` provides functions for going back and forth between mm-of-paper and projected coordinates given by map's georeferencing. And is obtained with calling `get_transform` on the map's `geo_referencing`-field.


## Dash-points and beziers
`omap-rs` ignores dash points on line and area objects. However, any line/area-object that `omap-rs` do not edit will be written back exactly as it was read, i.e. dash points will be preserved. The same is true for beziers. All line/area-objects are converted from beziers to LineString/Polygon on reading from file, but any object that is not edited will be written back as they were read, preserving beziers.
Added or edited objects can be chosen to be written back as beziers by toggeling the bezier bool in all line/area-objects.

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
    let mut map = Omap::default_15_000(
        proj_center,
        CrsType::Epsg(crs_epsg_code),
        map_center_elevation_meters,
    ).unwrap();

    // T
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

    // The Symbol set holds `Rc`s (owning pointers) of the symbols (which again hold weak
    // The Objects hold
    let erosion_gully = map
        .symbols
        .get_symbol_by_code(Code::new(107, 0, 0))
        .unwrap()
        .downgrade();

    // O-mapper makes no difference between line objects and area objects, but we do
    let mut ls = LineObject::new(
        WeakLinePathSymbol::try_from(erosion_gully).unwrap(),
        // geometry coordinates are always in mm of paper
        LineString::new(vec![Coord { x: 0., y: 0. }, Coord { x: 200., y: 100. }]),
    );
    // Let's convert this LineString to a Cubic bezier when writing to file
    ls.write_as_bezier = true;
    // Object tags
    ls.tags
        .insert("Some Key".to_string(), "My value".to_string());

    // A map can have multiple parts let's add the object to the first one
    map.parts.get_map_part_by_index_mut(0).unwrap().add_object(ls);

    // O-mapper makes no difference between combined line symbols and combined area symbols, but we do
    // This will debug-print a `CombinedLineSymbol`
    if let Some(s) = map.symbols.get_symbol_by_name("Railway, Olive background") {
        println!("{:?}", s);
    }

    map.write_to_file("./test_write.omap")
}
```