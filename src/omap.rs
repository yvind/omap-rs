use crate::{MapObject, OmapResult, Scale};
use geo_types::Coord;
use log::{log, Level};

use std::io::{BufWriter, Write};
use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

/// Struct representing an Orienteering map
/// ALL COORDINATES ARE RELATIVE THE ref_point
/// If epsg.is_some() the map is written georefrenced
/// else it is written in Local space
pub struct Omap {
    grivation: f32,
    scale: Scale,
    epsg: Option<u16>,
    ref_point: Coord,

    objects: Vec<MapObject>,
}

impl Omap {
    pub fn new(georef_point: Coord, epsg_crs: Option<u16>, scale: Scale) -> Self {
        // should use a magnetic model to figure out the declination (angle between true north and magnetic north) at the ref_point
        // and proj4rs for the convergence (angle between true north and grid north)

        // the grivation (angle between grid north and magnetic north) must be used when calulating map coords as the axes are magnetic

        Omap {
            grivation: 0.,
            scale,
            epsg: epsg_crs,
            ref_point: georef_point,
            objects: vec![],
        }
    }

    pub fn add_object(&mut self, obj: MapObject) {
        self.objects.push(obj);
    }

    pub fn write_to_file(
        self,
        filename: &OsStr,
        dir: &Path,
        bezier_error: Option<f64>,
    ) -> OmapResult<()> {
        let mut filepath = PathBuf::from(dir);
        filepath.push(filename);
        filepath.set_extension("omap");

        let f = File::create(&filepath)?;
        let mut f = BufWriter::new(f);

        self.write_header(&mut f)?;
        Self::write_colors_symbols(&mut f)?;
        self.write_objects(&mut f, bezier_error)?;
        Self::write_end_of_file(&mut f)?;
        Ok(())
    }

    fn write_header(&self, f: &mut BufWriter<File>) -> OmapResult<()> {
        match self.scale {
            Scale::S15_000 => (),
            _ => log!(Level::Warn, "Only 1:15_000 supported yet"),
        }

        f.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<map xmlns=\"http://openorienteering.org/apps/mapper/xml/v2\" version=\"9\">\n<notes></notes>\n")?;

        if self.epsg.is_some() {
            log!(Level::Warn, "Writing georefrenced files not yet supported");
        }

        f.write_all(format!("<georeferencing scale=\"15000\"><projected_crs id=\"Local\"><ref_point x=\"{}\" y=\"{}\"/></projected_crs></georeferencing>\n", self.ref_point.x, self.ref_point.y).as_bytes())?;
        Ok(())
    }

    fn write_colors_symbols(f: &mut BufWriter<File>) -> OmapResult<()> {
        f.write_all(include_str!("colors_and_symbols_omap.txt").as_bytes())?;
        Ok(())
    }

    fn write_objects(self, f: &mut BufWriter<File>, bezier_error: Option<f64>) -> OmapResult<()> {
        f.write_all(
            format!(
                "<parts count=\"1\" current=\"0\">\n<part name=\"map\"><objects count=\"{}\">\n",
                self.objects.len()
            )
            .as_bytes(),
        )?;

        for object in self.objects.into_iter() {
            object.write_to_map(f, bezier_error, self.scale, self.grivation)?;
        }

        f.write_all(b"</objects></part>\n</parts>\n")?;
        Ok(())
    }

    fn write_end_of_file(f: &mut BufWriter<File>) -> OmapResult<()> {
        f.write_all(b"<templates count=\"0\" first_front_template=\"0\">\n<defaults use_meters_per_pixel=\"true\" meters_per_pixel=\"0\" dpi=\"0\" scale=\"0\"/></templates>\n<view>\n")?;
        f.write_all(b"<grid color=\"#646464\" display=\"0\" alignment=\"0\" additional_rotation=\"0\" unit=\"1\" h_spacing=\"500\" v_spacing=\"500\" h_offset=\"0\" v_offset=\"0\" snapping_enabled=\"true\"/>\n")?;
        f.write_all(b"<map_view zoom=\"1\" position_x=\"0\" position_y=\"0\"><map opacity=\"1\" visible=\"true\"/><templates count=\"0\"/></map_view>\n</view>\n</barrier>\n</map>")?;
        Ok(())
    }
}
