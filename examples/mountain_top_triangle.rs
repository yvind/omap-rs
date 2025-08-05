use geo_types::{Coord, Point};
use omap::writer::{
    objects::{PointObject, TagTrait},
    symbols::PointSymbol,
    OmapWriter, Scale,
};
use std::{path::PathBuf, str::FromStr};

const GALDHOPIGGEN: Coord = Coord {
    x: 463_562.5,
    y: 6_833_872.7,
};

const DUMHOE: Coord = Coord {
    x: 460_027.4,
    y: 6_834_067.1,
};

const BUKKEHOE: Coord = Coord {
    x: 461_063.1,
    y: 6_830_322.9,
};

fn main() {
    let map_center = DUMHOE;

    let map_center_elevation_meters = 2_182.;
    let crs_epsg_code = 25832;

    let mut omap = OmapWriter::new(
        map_center,
        Scale::S15_000,
        Some(crs_epsg_code),
        Some(map_center_elevation_meters),
    )
    .expect("Could not make map with the given CRS-code");

    let ghp_point = Point(GALDHOPIGGEN - map_center);
    let mut ghp_object = PointObject::from_point(ghp_point, PointSymbol::SpotHeight, 0.);
    ghp_object.add_elevation_tag(2469.);

    let dh_point = Point(DUMHOE - map_center);
    let mut dh_object = PointObject::from_point(dh_point, PointSymbol::SpotHeight, 0.);
    dh_object.add_elevation_tag(2182.);

    let bh_point = Point(BUKKEHOE - map_center);
    let mut bh_object = PointObject::from_point(bh_point, PointSymbol::SpotHeight, 0.);
    bh_object.add_elevation_tag(2314.);

    omap.add_object(ghp_object);
    omap.add_object(dh_object);
    omap.add_object(bh_object);

    omap.write_to_file(
        PathBuf::from_str("./mountain_top_triangle.omap").unwrap(),
        Default::default(),
    )
    .expect("Could not write to file");
}
