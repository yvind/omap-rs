use geo_types::{Coord, LineString, Point, Polygon};
use omap::{
    objects::{AreaObject, LineObject, PointObject, TagTrait, TextObject},
    symbols::{AreaSymbol, LineSymbol, PointSymbol, TextSymbol},
    Omap, Scale,
};
use std::{path::PathBuf, str::FromStr};

fn main() {
    let map_center = Coord {
        x: 323_877.,
        y: 6_399_005.,
    };
    let map_center_elevation_meters = 100.;
    let crs_epsg_code = 3006;

    let mut omap = Omap::new(
        map_center,
        Scale::S15_000,
        Some(crs_epsg_code),
        Some(map_center_elevation_meters),
    )
    .expect("Could not make map with the given CRS-code");

    // coordinates of geometry are in the same units as the map_center, but relative the map_center
    let polygon = Polygon::new(
        LineString::new(vec![
            Coord { x: -50., y: -50. },
            Coord { x: -50., y: 50. },
            Coord { x: 50., y: 50. },
            Coord { x: 50., y: -50. },
            Coord { x: -50., y: -50. },
        ]),
        vec![],
    );
    let mut area_object =
        AreaObject::from_polygon(polygon, AreaSymbol::RoughVineyard, 45.0_f64.to_radians());
    area_object.add_tag("tag_key", "tag_value");

    let line_string = LineString::new(vec![
        Coord { x: -60., y: 20. },
        Coord { x: -20., y: 25. },
        Coord { x: 0., y: 27.5 },
        Coord { x: 20., y: 26. },
        Coord { x: 40., y: 22.5 },
        Coord { x: 60., y: 20. },
        Coord { x: 60., y: -20. },
        Coord { x: -60., y: -20. },
    ]);
    let mut line_object = LineObject::from_line_string(line_string, LineSymbol::Contour);
    line_object.add_elevation_tag(20.);

    let point = Point::new(0.0_f64, 0.0_f64);
    let point_object = PointObject::from_point(
        point,
        PointSymbol::ElongatedDotKnoll,
        -45.0_f64.to_radians(),
    );

    let text_point = Point::new(0.0_f64, -30.0_f64);
    let text = "some text".to_string();
    let text_object = TextObject::from_point(text_point, TextSymbol::SpotHeight, text);

    omap.add_object(area_object);
    omap.add_object(line_object);
    omap.add_object(point_object);
    omap.add_object(text_object);

    let max_bezier_deviation_meters = 2.5;

    let bez_error = omap::BezierError::new(Some(max_bezier_deviation_meters), None);

    omap.write_to_file(
        PathBuf::from_str("./simple_example.omap").unwrap(),
        bez_error,
    )
    .expect("Could not write to file");
}
