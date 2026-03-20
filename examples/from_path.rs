use geo_types::{Coord, LineString};
#[cfg(feature = "geo_ref")]
use omap::geo_referencing::{CrsType, GeoRef};
use omap::{Code, Error, Omap, colors::Color, objects::LineObject, symbols::Symbol};

fn main() -> Result<(), Error> {
    let mut map = Omap::from_path("./test.omap")?;

    #[cfg(feature = "geo_ref")]
    {
        let proj_center = Coord {
            x: 463_575.5,
            y: 6_833_849.6,
        };
        let map_center_elevation_meters = 2_469.;
        let crs_epsg_code = 25832;
        let gr = GeoRef::initialize(
            proj_center,
            CrsType::Epsg(crs_epsg_code),
            map_center_elevation_meters,
            map.geo_referencing.scale_denominator,
        )?;
        map.geo_referencing = gr;
    };

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
        erosion_gully.try_into().unwrap(),
        // geometry coordinates are always in mm of paper
        LineString::new(vec![Coord { x: 0., y: 0. }, Coord { x: 200., y: 100. }]),
    );
    ls.tags
        .insert("Some Key".to_string(), "My value".to_string());

    map.parts.0[0].add_object(ls);

    println!("\nCombined Line symbols:");
    for symbol in map.symbols.iter() {
        if let Symbol::CombinedLine(s) = symbol {
            println!("{}", s.borrow().common.name)
        }
    }
    println!("\nTemplates:");
    for template in map.templates.iter() {
        println!("{:?}\n", template);
    }

    map.write_to_file("./test_write.omap")
}
