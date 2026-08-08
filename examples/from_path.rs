#![expect(
    clippy::print_stdout,
    reason = "the example intentionally displays the map data it reads"
)]
#![expect(
    clippy::unwrap_used,
    reason = "the example input is expected to contain nonempty objects and known symbols"
)]
#![expect(
    clippy::expect_used,
    reason = "the example input is expected to give the named symbol its documented type"
)]

use std::{cell::RefCell, rc::Weak};

use geo_types::{Coord, LineString};
#[cfg(feature = "geo_ref")]
use omap::NonNegativeF64;
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
        let old_transform = map.geo_referencing.create_transform();

        let mut mean_pos = Coord::zero();
        let mut num_coords = 0;
        for obj in map.iter_all_objects() {
            match obj {
                MapObject::Point(object) => {
                    mean_pos = mean_pos + object.geometry().0;
                    num_coords += 1;
                }
                MapObject::Line(object) => {
                    let geometry = object.flatten(NonNegativeF64::clamped_from(0.1))?;
                    let line_string = geometry.geometry();
                    mean_pos = mean_pos
                        + line_string
                            .0
                            .iter()
                            .copied()
                            .reduce(|sum, c| sum + c)
                            .unwrap();
                    num_coords += line_string.0.len();
                }
                MapObject::Area(object) => {
                    let geometry = object.flatten(NonNegativeF64::clamped_from(0.1))?;
                    let exterior = geometry.exterior().geometry();
                    mean_pos =
                        mean_pos + exterior.0.iter().copied().reduce(|sum, c| sum + c).unwrap();
                    num_coords += exterior.0.len();
                }
                MapObject::Text(object) => {
                    match object.geometry() {
                        TextGeometry::SingleAnchor(coord) => mean_pos = mean_pos + *coord,
                        TextGeometry::WrapBox(wrap_box) => mean_pos = mean_pos + wrap_box.anchor,
                    }
                    num_coords += 1;
                }
            }
        }
        mean_pos = mean_pos / num_coords as f64;

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
        let new_transform = map.geo_referencing.create_transform();

        // transfrom every object out of the old map space to projected coords
        // and from projected coord to the new map space
        // NB! If the new projection were different than the old (not just an affine transformation),
        // the geo_ref feature must be activated
        map.try_transform_between(&old_transform, &new_transform)
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
        .symbol_by_code(Code::new(107, 0, 0))
        .unwrap()
        .unwrap()
        .downgrade();

    let mut ls = LineObject::new(
        WeakLinePathSymbol::try_from(erosion_gully).unwrap(),
        // geometry coordinates are always in mm of paper
        LineString::new(vec![Coord { x: -60., y: -50. }, Coord { x: 60., y: -50. }]),
    );
    ls.tags.insert("Some Key".to_owned(), "My value".to_owned());

    map.parts.get_mut(0).unwrap().add_object(ls);

    let weak_symbol = map
        .symbols
        .symbol_by_name("Contour value")
        .unwrap()
        .unwrap()
        .downgrade();

    let ts = TextObject::new(
        Weak::<RefCell<TextSymbol>>::try_from(weak_symbol)
            .expect("The symbol type of Contour value is not Text"),
        TextGeometry::SingleAnchor(Coord { x: 0., y: 0. }),
        "This is the middle of the map".to_owned(),
    );
    map.parts.get_mut(0).unwrap().add_object(ts);

    println!("Does the map pass validation: {:?}", map.validate());

    map.to_file("./from_path_out.omap")
}
