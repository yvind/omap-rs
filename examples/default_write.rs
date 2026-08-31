#![expect(
    clippy::print_stdout,
    reason = "the example intentionally displays the map data it reads"
)]
#![expect(
    clippy::unwrap_used,
    reason = "the bundled default map is known to contain these symbols and colors"
)]

use geo_types::{Coord, LineString};
#[cfg(feature = "geo_ref")]
use omap::geo_referencing::CrsType;
use omap::{
    Code, Error, Omap,
    colors::Color,
    objects::LineObject,
    symbols::{PublicOrPrivateSymbol, Symbol},
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
        Omap::default_15_000_geo_referenced(
            proj_center,
            CrsType::Epsg(crs_epsg_code),
            map_center_elevation_meters,
        )?
    };
    #[cfg(not(feature = "geo_ref"))]
    let mut map = Omap::default_15_000()?;

    for color in map.colors.values() {
        match color {
            Color::SpotColor(spot) => {
                println!("{} with spot name {}", spot.color_name, spot.spotcolor_name);
            }
            Color::MixedColor(mixed) => println!("{}", mixed.color_name),
        }
    }

    let erosion_gully = map
        .symbols
        .find_by_code(Code::new(107, 0, 0))
        .and_then(|symbol| symbol.as_line_path())
        .unwrap();

    let mut ls = LineObject::new(
        Some(erosion_gully),
        // geometry coordinates are always in mm of paper
        LineString::new(vec![Coord { x: 0., y: 0. }, Coord { x: 200., y: 100. }]),
    );
    ls.tags_mut()
        .insert("Some Key".to_owned(), "My value".to_owned());

    map.parts.get_mut(0).unwrap().add_object(ls);

    println!("\nCombined Line symbols:");
    for symbol in map.symbols.values() {
        if let Symbol::CombinedLine(s) = symbol {
            println!("{}", s.common.name);
        }
    }
    if let Some(s) = map.symbols.find_by_name("Railway, Olive background") {
        println!("{:?}", s.symbol());
    }

    let mut num = 0;
    for symbol in map.symbols.values() {
        if let Symbol::Line(ls) = symbol {
            if let Some(_ss_) = &ls.start_symbol {
                num += 1;
            }
            if let Some(_ms_) = &ls.mid_symbol {
                num += 1;
            }
            if let Some(_ds_) = &ls.dash_symbol {
                num += 1;
            }
            if let Some(_es_) = &ls.end_symbol {
                num += 1;
            }
        }
        if let Symbol::CombinedLine(s) = symbol {
            for part in s.components() {
                if let PublicOrPrivateSymbol::Private(ls) = part {
                    if let Some(_ss_) = &ls.start_symbol {
                        num += 1;
                    }
                    if let Some(_ms_) = &ls.mid_symbol {
                        num += 1;
                    }
                    if let Some(_ds_) = &ls.dash_symbol {
                        num += 1;
                    }
                    if let Some(_es_) = &ls.end_symbol {
                        num += 1;
                    }
                }
            }
        }
    }
    println!("\nNumber of sub symbols in line symbols: {num}");

    println!("Does the map pass validation: {:?}", map.validate());

    map.to_file("./test_write.omap")
}
