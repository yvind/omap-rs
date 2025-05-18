# Omap-rs
A library for writing `geo_types`-geometries to OpenOrienteering Mapper's .omap files.  

The files are automatically georeferenced (including scale factors) and magnetic north aligned (using the current WMM, date and map-location) if a Coordinate Reference System is provided (by EPSG code). 

Scales 1:15_000 and 1:10_000 are supported.

## Example

```Rust
use omap::{
    objects::{AreaObject, LineObject, PointObject, TextObject, TagTrait},
    symbols::{AreaSymbol, LineSymbol, PointSymbol, TextSymbol},
    Omap, Scale,
};
use geo_types::{Coord, LineString, Polygon, Point};
use std::{path::PathBuf, str::FromStr};

let map_center = Coord {x: 463_575.5, y: 6_833_849.6};

let mut omap = Omap::new(map_center, Scale::S15_000, Some(25832), Some(2_469.)).expect("Could not make map with the given CRS-code");

// coordinates of geometry are relative the map_center
let polygon = Polygon::new(
    LineString::new(vec![
        Coord {x: -50., y: -50.},
        Coord {x: -50., y: 50.},
        Coord {x: 50., y: 50.},
        Coord {x: 50., y: -50.},
        Coord {x: -50., y: -50.},
    ]), vec![]);
let mut area_object = AreaObject::from_polygon(polygon, AreaSymbol::RoughVineyard, 45.0_f64.to_radians());
area_object.add_tag("tag_key", "tag_value");

let line_string = LineString::new(
        vec![
            Coord {x: -60., y: 20.},
            Coord {x: -20., y: 25.},
            Coord {x: 0., y: 27.5},
            Coord {x: 20., y: 25.},
            Coord {x: 60., y: 20.},
            Coord {x: 60., y: -20.},
            Coord {x: -60., y: -20.},
        ]
    );
let mut line_object = LineObject::from_line_string(line_string, LineSymbol::Contour);
line_object.add_elevation_tag(20.);

let point = Point::new(0.0_f64, 0.0_f64);
let point_object = PointObject::from_point(point, PointSymbol::ElongatedDotKnoll, -45.0_f64.to_radians());

let text_point = Point::new(0.0_f64, -30.0_f64);
let text = "some text".to_string();
let text_object = TextObject::from_point(text_point, TextSymbol::SpotHeight, text);

omap.add_object(area_object);
omap.add_object(line_object);
omap.add_object(point_object);
omap.add_object(text_object);

let max_bezier_deviation = 2.5;
omap.write_to_file(PathBuf::from_str("./my_map.omap").unwrap(), Some(max_bezier_deviation)).expect("Could not write to file");
```