# Omap-rs
[![crates.io version](https://img.shields.io/crates/v/omap.svg)](https://crates.io/crates/omap)
[![docs.rs docs](https://docs.rs/omap/badge.svg)](https://docs.rs/omap)  

A library for working with OpenOrienteering Mapper's .omap files.

For writing new files you can either start with a completely empty map `Omap::new` or use one of the provided templates `Omap::default_15_000`, `Omap::default_10_000` or `Omap::default_4_000`.
Or you can start from an already existing file with `Omap::from_path`.

## Geo-referencing
With the `geo_ref`-feature automatic geo-referencing with magnetic north and scale factor calculation is enabled and done with the `omap::geo_referencing::GeoRef::initialize` function. \
It is not enabled by default because of the extra dependencies needed (Proj4rs for coordinate projections, WMM for magnetic north calcualtion and Chrono for time as the magnetic north changes over time). Without this feature the georeferencing must be done by hand.

**NB!** if you change any field (or the entire thing) in the map's `geo_referencing`-field then all the map objects projected/geographic positions will change as their coordinates are given in mm-of-paper and remain untouched.
The best practice is to set the map's geo referencing before adding any objects.

`omap::geo_referencing::MapTransform` provides functions for going back and forth between mm-of-paper and projected coordinates given by the map's georeferencing. Obtain one by calling `create_transform` on the map's `geo_referencing` field.

`MapTransform::transform_between` can be used to keep objects and non-georeferenced templates at the same real-world positions after changing the map's georeferencing. Without `geo_ref`, both maps must use the same projection; with `geo_ref`, differing projections are converted automatically.

With the `geo_ref`-feature the same `MapTransform` also reaches WGS84 (`x` longitude, `y` latitude, in degrees) without a second projection dependency: `to_wgs84`, `to_wgs84_polygon`, `to_wgs84_bezierpath` and the rest of the family go from mm-of-paper all the way to degrees, and the `from_wgs84`-family comes back.
They chain the paper ↔ projected step with a projection between the map's CRS and WGS84, resolved through `CrsType::to_crs_def`, the same resolution `GeoRef::initialize` uses.
That projection is compiled on first use and kept, so hold on to the transform instead of calling `create_transform` per object.

## Dash-points and beziers

Line and area objects store their exact mixed straight/cubic geometry as a `BezierPath` or `BezierPolygon`. Per-vertex metadata includes forced dash points, including both endpoints of an open path, and this geometry
is always used when writing.

Use `BezierPath::fit_line_string(line_string, error)` or
`BezierPolygon::fit_polygon(polygon, error)` to fit smooth cubic geometry
without depending directly on `linestring2bezier`:

```rust
let line = LineObject::new(
    line_symbol,
    BezierPath::fit_line_string(line_string, error)?,
);
let area = AreaObject::new(
    area_symbol,
    BezierPolygon::fit_polygon(polygon, error)?,
);
```

Passing a `LineString` or `Polygon` directly to `LineObject::new` or
`AreaObject::new` instead preserves the input as straight segments.

`flatten(error)` returns an owned `FlattenedPath` or `FlattenedPolygon` with
dash-point metadata aligned to every flattened coordinate. Flattened geometry
is not cached by the crate. `replace_with_flattened` and `flatten_in_place`
explicitly discard curves by replacing them with straight segments.

Empty line and area geometries can remain in a map part while they are being
edited, but they are omitted when an `.omap` file is written. Consequently,
writing and reading a map removes empty line and area objects.

## Example

```rust
use geo_types::{Coord, LineString};
use omap::{
    Code, Error, Omap,
    colors::Color,
    geo_referencing::CrsType,
    objects::LineObject,
    symbols::LinePathSymbolId,
};

fn main() -> Result<(), Error> {
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

    // Iterate through the colors, in priority order
    for color in map.colors.values() {
        match color {
            // Colors are split between `SpotColor` which defines new colors
            Color::SpotColor(spot) => {
                println!("{} with spot name {}", spot.color_name, spot.spotcolor_name);
            }
            // Or `MixedColor` which are made up of weighted `SpotColor`-components
            Color::MixedColor(mixed) => println!("{}", mixed.color_name),
        }
    }

    // The symbol set owns the symbols and hands out `SymbolId` handles. A handle
    // is a `Copy` integer that keeps naming the same symbol across sorting, and
    // stops resolving once the symbol is removed.
    let erosion_gully = map
        .symbols
        .id_by_code(Code::new(107, 0, 0))
        .expect("the default symbol set contains erosion gully");

    // O-mapper makes no difference between line objects and area objects, but we do.
    let mut ls = LineObject::new(
        Some(LinePathSymbolId::try_from(erosion_gully)?),
        // LineStrings become straight Bézier segments. Geometry
        // coordinates are always in mm of paper.
        LineString::new(vec![Coord { x: 0., y: 0. }, Coord { x: 200., y: 100. }]),
    );
    // Add some tags to the object
    ls.tags.insert("Some Key".to_string(), "My value".to_string());

    // A map can have multiple parts let's add the object to the first one
    map.parts
        .get_mut(0)
        .expect("a default map contains one map part")
        .add_object(ls);

    // O-mapper makes no difference between combined line symbols and combined area symbols, but we do
    // This will debug-print a `CombinedLineSymbol`
    if let Some(s) = map
        .symbols
        .symbol_by_name("Railway, Olive background")
    {
        dbg!(s);
    }

    map.write_to_file("./test_write.omap")
}
```
