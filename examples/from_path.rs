use std::{cell::RefCell, rc::Weak};

use geo_types::{Coord, LineString};
#[cfg(feature = "geo_ref")]
use omap::geo_referencing::GeoRef;
#[cfg(feature = "geo_ref")]
use omap::objects::MapObject;
use omap::{
    Code, Error, Omap,
    colors::Color,
    objects::{LineObject, TextGeometry, TextObject},
    symbols::{TextSymbol, WeakLinePathSymbol},
};

fn main() -> Result<(), Error> {
    let mut map = Omap::from_path("./example_data/from_path.omap")?;

    #[cfg(feature = "geo_ref")]
    {
        // we want to move the map center to the average position of all objects
        let old_transform = map.geo_referencing.get_transform();

        let mut mean_pos = Coord::zero();
        let mut num_coords = 0;
        for obj in map.parts.iter().flat_map(|part| part.iter_all_objects()) {
            match obj {
                MapObject::Point(object) => {
                    mean_pos = mean_pos + object.get_geometry().0;
                    num_coords += 1;
                }
                MapObject::Line(object) => {
                    mean_pos = mean_pos
                        + object
                            .get_geometry()
                            .0
                            .iter()
                            .copied()
                            .reduce(|sum, c| sum + c)
                            .unwrap();
                    num_coords += object.get_geometry().0.len();
                }
                MapObject::Area(object) => {
                    mean_pos = mean_pos
                        + object
                            .get_geometry()
                            .exterior()
                            .0
                            .iter()
                            .copied()
                            .reduce(|sum, c| sum + c)
                            .unwrap();
                    num_coords += object.get_geometry().exterior().0.len();
                }
                MapObject::Text(object) => {
                    match object.get_geometry() {
                        TextGeometry::SingleAnchor(coord) => mean_pos = mean_pos + *coord,
                        TextGeometry::WrapBox(wrap_box) => mean_pos = mean_pos + wrap_box.anchor,
                    }
                    num_coords += 1;
                }
            }
        }
        mean_pos = mean_pos / num_coords as f64;
        dbg!(mean_pos);

        // now transform that into projected coords
        let mean_proj_pos = old_transform.to_projected(mean_pos);

        // get the new georef info for that position
        let new_gr = GeoRef::initialize(
            mean_proj_pos,
            map.geo_referencing.crs_type,
            2_469.,
            map.geo_referencing.scale_denominator,
        )
        .unwrap();

        // assign the new info
        map.geo_referencing = new_gr;

        // get the new map transform
        let new_transform = map.geo_referencing.get_transform();

        // transfrom every object out of the old map space to projected coords
        // and from projected coord to the new map space
        // NB! If the new projection were different than the old,
        // a transfrom between projections using a proj library like proj-core would be needed
        // and this function would return Err
        map.apply_affine_between(&old_transform, &new_transform)
            .unwrap();
    };

    println!("Map colors in order:");
    for color in map.colors.iter() {
        match color {
            Color::SpotColor(ref_cell) => {
                let b = ref_cell.try_borrow().unwrap();
                println!("{} with spot name {}", b.color_name, b.spotcolor_name);
            }
            Color::MixedColor(ref_cell) => {
                println!("{}", ref_cell.try_borrow().unwrap().color_name);
            }
        }
    }

    let erosion_gully = map
        .symbols
        .get_symbol_by_code(Code::new(107, 0, 0))
        .unwrap()
        .downgrade();

    let mut ls = LineObject::new(
        WeakLinePathSymbol::try_from(erosion_gully).unwrap(),
        // geometry coordinates are always in mm of paper
        LineString::new(vec![Coord { x: -60., y: -50. }, Coord { x: 60., y: -50. }]),
    );
    ls.tags
        .insert("Some Key".to_string(), "My value".to_string());

    map.parts.0[0].add_object(ls);

    let weak_symbol = map
        .symbols
        .get_symbol_by_name("Contour value")
        .unwrap()
        .downgrade();

    let ts = TextObject::new(
        Weak::<RefCell<TextSymbol>>::try_from(weak_symbol)
            .expect("The symbol type of Contour value is not Text"),
        TextGeometry::SingleAnchor(Coord { x: 0., y: 0. }),
        "This is the middle of the map".to_string(),
    );
    map.parts.0[0].add_object(ts);

    map.write_to_file("./from_path_out.omap")
}
