use geo_types::{Coord, LineString};
#[cfg(feature = "geo_ref")]
use omap::geo_referencing::CrsType;
use omap::{
    Code, Error, Omap,
    colors::Color,
    objects::LineObject,
    symbols::{PubOrPrivSymbol, Symbol},
};

fn main() -> Result<(), Error> {
    #[cfg(feature = "geo_ref")]
    let mut map = {
        let proj_center = Coord {
            x: 463_575.5,
            y: 6_833_849.6,
        };
        let map_center_elevation_meters = 2_469.;
        let crs_epsg_code = 25832;
        Omap::default_15_000(
            proj_center,
            CrsType::Epsg(crs_epsg_code),
            map_center_elevation_meters,
        )?
    };
    #[cfg(not(feature = "geo_ref"))]
    let mut map = Omap::default_15_000()?;

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
            println!("{}", s.borrow().common.name);
        }
    }
    if let Some(s) = map.symbols.get_symbol_by_name("Railway, Olive background") {
        println!("{:?}", s);
    }

    let mut num = 0;
    for symbol in map.symbols.iter() {
        if let Symbol::Line(s) = symbol {
            let borrowed = s.borrow();

            if let Some(_ss_) = &borrowed.start_symbol {
                num += 1;
            }
            if let Some(_ms_) = &borrowed.mid_symbol {
                num += 1;
            }
            if let Some(_ds_) = &borrowed.dash_symbol {
                num += 1;
            }
            if let Some(_es_) = &borrowed.end_symbol {
                num += 1;
            }
        }
        if let Symbol::CombinedLine(s) = symbol {
            let borrowed = s.borrow();

            for part in borrowed.components() {
                if let PubOrPrivSymbol::Private(s) = part {
                    if let Some(_ss_) = &s.start_symbol {
                        num += 1;
                    }
                    if let Some(_ms_) = &s.mid_symbol {
                        num += 1;
                    }
                    if let Some(_ds_) = &s.dash_symbol {
                        num += 1;
                    }
                    if let Some(_es_) = &s.end_symbol {
                        num += 1;
                    }
                }
            }
        }
    }
    println!("\nNumber of sub symbols in line symbols: {num}");

    map.write_to_file("./test_write.omap")
}
