use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use crate::{Error, Result, utils::parse_attr, utils::try_get_attr};

/// A 3×3 matrix stored in row-major order.
#[derive(Debug, Clone)]
pub struct Matrix3x3(pub [f64; 9]);

/// The `<transformations>` block for a non-georeferenced template.
#[derive(Debug, Clone)]
pub struct Transformations {
    /// Adjustment state.
    pub adjustment: AdjustmentState,
    /// The currently active transform (role="active").
    pub active_transform: TemplateTransform,
    /// The other (inactive) transform (role="other").
    pub other_transform: TemplateTransform,
    /// Pass-points used for adjustment.
    pub passpoints: Vec<PassPoint>,
    /// The 3×3 map-to-template matrix, row-major.
    pub map_to_template: Option<Matrix3x3>,
    /// The 3×3 template-to-map matrix, row-major.
    pub template_to_map: Option<Matrix3x3>,
    /// The 3×3 template-to-map matrix for the "other" transform, row-major.
    pub template_to_map_other: Option<Matrix3x3>,
}

/// Whether the adjustment is applied, dirty, or neither.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdjustmentState {
    /// No adjustment has been applied.
    NoAdjustment,
    /// Adjustment successfully applied (`adjusted="true"`).
    Adjusted,
    /// Adjustment needs recalculation (`adjustment_dirty="true"`).
    AdjustmentDirty,
}

/// Parameters for a single `<transformation>` element.
///
/// Mirrors the C++ `TemplateTransform` struct.
#[derive(Debug, Clone)]
pub struct TemplateTransform {
    /// Template position in 1/1000 mm.
    pub template_pos: Coord<i32>,
    /// Rotation in radians (positive = counter-clockwise).
    pub template_rotation: f64,
    /// Template scaling (< 1 shrinks).
    pub template_scale: Coord,
    /// Shear component (rare, usually 0).
    pub template_shear: f64,
}

impl Default for TemplateTransform {
    fn default() -> Self {
        Self {
            template_pos: Coord::zero(),
            template_rotation: 0.,
            template_scale: Coord { x: 1., y: 1. },
            template_shear: 0.,
        }
    }
}

/// A pass-point relating source (template) coords to destination (map) coords.
#[derive(Debug, Clone)]
pub struct PassPoint {
    pub src_coord: Coord,
    pub dest_coord: Coord,
    pub calculated_coord: Coord,
}

impl PassPoint {
    pub fn error(&self) -> f64 {
        let diff = self.calculated_coord - self.dest_coord;
        (diff.x.powi(2) + diff.y.powi(2)).sqrt()
    }
}

impl Transformations {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        bs: &BytesStart<'_>,
    ) -> Result<Self> {
        let mut adjustment = AdjustmentState::NoAdjustment;

        for attr in bs.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"adjusted" => {
                    if attr.as_bool().unwrap_or(false) {
                        adjustment = AdjustmentState::Adjusted;
                    }
                }
                b"adjustment_dirty" => {
                    if attr.as_bool().unwrap_or(false) {
                        adjustment = AdjustmentState::AdjustmentDirty;
                    }
                }
                _ => {}
            }
        }

        let mut active_transform = TemplateTransform::default();
        let mut other_transform = TemplateTransform::default();
        let mut passpoints = Vec::new();
        let mut map_to_template = None;
        let mut template_to_map = None;
        let mut template_to_map_other = None;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(child) => match child.local_name().as_ref() {
                    b"transformation" => {
                        let t = TemplateTransform::parse(&child);
                        if let Some(role) = child
                            .attributes()
                            .filter_map(std::result::Result::ok)
                            .find(|a| a.key.local_name().as_ref() == b"role")
                        {
                            match role.value.as_ref() {
                                b"other" => other_transform = t,
                                _ => active_transform = t,
                            }
                        }
                    }
                    b"passpoint" => {
                        passpoints.push(PassPoint::parse(reader)?);
                    }
                    b"matrix" => {
                        let m = Matrix3x3::parse(reader, &child)?;
                        if let Some(role) = child
                            .attributes()
                            .filter_map(std::result::Result::ok)
                            .find(|a| a.key.local_name().as_ref() == b"role")
                        {
                            match role.value.as_ref() {
                                b"map_to_template" => map_to_template = Some(m),
                                b"template_to_map" => template_to_map = Some(m),
                                b"template_to_map_other" => template_to_map_other = Some(m),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                },
                Event::End(ref be) if be.local_name().as_ref() == b"transformations" => break,
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in transformations".into(),
                    ));
                }
                _ => {}
            }
        }

        Ok(Transformations {
            adjustment,
            active_transform,
            other_transform,
            passpoints,
            map_to_template,
            template_to_map,
            template_to_map_other,
        })
    }
}

impl TemplateTransform {
    pub(crate) fn parse(bs: &BytesStart<'_>) -> Self {
        let mut t = Self::default();
        for attr in bs.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"x" => t.template_pos.x = parse_attr(attr.value).unwrap_or(0),
                b"y" => t.template_pos.x = parse_attr(attr.value).unwrap_or(0),
                b"rotation" => t.template_rotation = parse_attr(attr.value).unwrap_or(0.),
                b"scale_x" => t.template_scale.x = parse_attr(attr.value).unwrap_or(1.),
                b"scale_y" => t.template_scale.y = parse_attr(attr.value).unwrap_or(1.),
                b"shear" => t.template_shear = parse_attr(attr.value).unwrap_or(0.),
                _ => {}
            }
        }
        t
    }

    pub(crate) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        role: &str,
    ) -> Result<()> {
        let mut start = BytesStart::new("transformation").with_attributes([
            ("role", role),
            ("x", self.template_pos.x.to_string().as_str()),
            ("y", self.template_pos.y.to_string().as_str()),
            ("scale_x", format!("{:.5}", self.template_scale.x).as_str()),
            ("scale_y", format!("{:.5}", self.template_scale.y).as_str()),
            (
                "rotation",
                format!("{:.5}", self.template_rotation).as_str(),
            ),
        ]);
        if self.template_shear != 0.0 {
            start.push_attribute(("shear", format!("{:.5}", self.template_shear).as_str()));
        }
        writer.write_event(Event::Empty(start))?;
        Ok(())
    }
}

impl PassPoint {
    pub(crate) fn parse<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<Self> {
        let mut src = Coord::zero();
        let mut dest = Coord::zero();
        let mut calc = Coord::zero();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref child) => match child.local_name().as_ref() {
                    b"source" => src = parse_inner_coord(reader)?,
                    b"destination" => dest = parse_inner_coord(reader)?,
                    b"calculated" => calc = parse_inner_coord(reader)?,
                    _ => {}
                },
                Event::End(ref be) if be.local_name().as_ref() == b"passpoint" => break,
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in passpoint".into(),
                    ));
                }
                _ => {}
            }
        }

        Ok(PassPoint {
            src_coord: src,
            dest_coord: dest,
            calculated_coord: calc,
        })
    }

    pub(crate) fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new("passpoint").with_attributes(
                [("error", format!("{:.5}", self.error()).as_str())],
            )))?;
        write_coord_wrapper(writer, "source", &self.src_coord)?;
        write_coord_wrapper(writer, "destination", &self.dest_coord)?;
        write_coord_wrapper(writer, "calculated", &self.calculated_coord)?;

        writer.write_event(Event::End(BytesEnd::new("passpoint")))?;
        Ok(())
    }
}

impl Matrix3x3 {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        _bs: &BytesStart<'_>,
    ) -> Result<Self> {
        let mut values = [0.; 9];
        let mut i = 0;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(child) if child.local_name().as_ref() == b"element" => {
                    values[i] = try_get_attr(&child, "value").unwrap_or(0.);
                    i += 1;
                }
                Event::End(be) if be.local_name().as_ref() == b"matrix" => break,
                Event::Eof => {
                    return Err(Error::ParseOmapFileError("Unexpected EOF in matrix".into()));
                }
                _ => {}
            }
        }

        Ok(Matrix3x3(values))
    }

    pub(crate) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        role: &str,
    ) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("matrix").with_attributes([
            ("role", role),
            ("n", "3"),
            ("m", "3"),
        ])))?;
        for val in self.0 {
            writer.write_event(Event::Empty(
                BytesStart::new("element")
                    .with_attributes([("value", format!("{}", val).as_str())]),
            ))?;
        }
        writer.write_event(Event::End(BytesEnd::new("matrix")))?;
        Ok(())
    }
}

fn write_coord_wrapper<W: std::io::Write>(
    writer: &mut Writer<W>,
    wrapper_name: &str,
    coord: &Coord,
) -> Result<()> {
    writer.write_event(Event::Start(BytesStart::new(wrapper_name)))?;
    writer.write_event(Event::Empty(BytesStart::new("coord").with_attributes([
        ("x", format!("{}", coord.x).as_str()),
        ("y", format!("{}", coord.y).as_str()),
    ])))?;
    writer.write_event(Event::End(BytesEnd::new(wrapper_name)))?;
    Ok(())
}

fn parse_inner_coord<R: std::io::BufRead, T: std::str::FromStr + geo_types::CoordNum>(
    reader: &mut Reader<R>,
) -> Result<Coord<T>> {
    let mut coord = Coord::zero();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(bs) if bs.local_name().as_ref() == b"coord" => {
                coord = Coord {
                    x: try_get_attr(&bs, "x").unwrap_or(T::zero()),
                    y: try_get_attr(&bs, "y").unwrap_or(T::zero()),
                };
            }
            Event::End(_) => break,
            Event::Eof => {
                return Err(Error::ParseOmapFileError(
                    "Unexpected EOF in coord wrapper".into(),
                ));
            }
            _ => {}
        }
    }
    Ok(coord)
}
