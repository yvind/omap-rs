use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use crate::{
    Error, OmapSection, Result,
    geo_referencing::AffineMapTransform,
    utils::{from_file_coords, parse_attr_raw, to_file_coords, try_get_attr_raw},
};

/// A 3×3 matrix stored in row-major order.
#[derive(Debug, Clone)]
pub struct Matrix3x3(pub [f64; 9]);

/// The `<transformations>` block for a non-georeferenced template.
#[derive(Debug, Clone)]
pub struct TemplateTransformations {
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
#[derive(Debug, Clone)]
pub struct TemplateTransform {
    /// Template position in mm of paper.
    pub template_pos: Coord,
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
        if self.calculated_coord == Coord::zero() {
            return -1.;
        }
        let diff = self.calculated_coord - self.dest_coord;
        (diff.x.powi(2) + diff.y.powi(2)).sqrt()
    }
}

impl TemplateTransformations {
    pub(crate) fn apply_affine(&mut self, transform: &AffineMapTransform) {
        self.active_transform.apply_affine(transform);
        self.other_transform.apply_affine(transform);
        for passpoint in &mut self.passpoints {
            passpoint.apply_affine(transform);
        }

        let file_transform = Matrix3x3(transform.file_coord_matrix());
        if let Some(inverse_file_transform) = file_transform.inverse_affine() {
            self.map_to_template = self
                .map_to_template
                .as_ref()
                .map(|matrix| matrix.multiply(&inverse_file_transform));
            self.template_to_map = self
                .template_to_map
                .as_ref()
                .map(|matrix| file_transform.multiply(matrix));
            self.template_to_map_other = self
                .template_to_map_other
                .as_ref()
                .map(|matrix| file_transform.multiply(matrix));
        } else {
            self.map_to_template = None;
            self.template_to_map = None;
            self.template_to_map_other = None;
            self.adjustment = AdjustmentState::AdjustmentDirty;
        }
    }

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
                        let m = Matrix3x3::parse(reader)?;
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
                Event::End(be) => {
                    if be.local_name().as_ref() == b"transformations" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::TemplateTransformations));
                }
                _ => {}
            }
        }

        Ok(TemplateTransformations {
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
    pub(crate) fn apply_affine(&mut self, transform: &AffineMapTransform) {
        self.template_pos = transform.apply(self.template_pos);
        self.template_rotation += transform.rotation_radians();
        self.template_scale.x *= transform.scale_factor();
        self.template_scale.y *= transform.scale_factor();
    }

    pub(crate) fn parse(bs: &BytesStart<'_>) -> Self {
        let mut t = Self::default();
        let mut pos = Coord::default();
        for attr in bs.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"x" => pos.x = parse_attr_raw::<i32>(attr.value).unwrap_or(0),
                b"y" => pos.y = parse_attr_raw::<i32>(attr.value).unwrap_or(0),
                b"rotation" => t.template_rotation = parse_attr_raw(attr.value).unwrap_or(0.),
                b"scale_x" => t.template_scale.x = parse_attr_raw(attr.value).unwrap_or(1.),
                b"scale_y" => t.template_scale.y = parse_attr_raw(attr.value).unwrap_or(1.),
                b"shear" => t.template_shear = parse_attr_raw(attr.value).unwrap_or(0.),
                _ => {}
            }
        }
        t.template_pos = from_file_coords(pos);
        t
    }

    pub(crate) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        role: &str,
    ) -> Result<()> {
        let pos = to_file_coords(self.template_pos)?;
        let mut start = BytesStart::new("transformation").with_attributes([
            ("role", role),
            ("x", pos.x.to_string().as_str()),
            ("y", pos.y.to_string().as_str()),
            ("scale_x", self.template_scale.x.to_string().as_str()),
            ("scale_y", self.template_scale.y.to_string().as_str()),
            ("rotation", self.template_rotation.to_string().as_str()),
        ]);
        if self.template_shear != 0.0 {
            start.push_attribute(("shear", self.template_shear.to_string().as_str()));
        }
        writer.write_event(Event::Empty(start))?;
        Ok(())
    }
}

impl PassPoint {
    pub(crate) fn apply_affine(&mut self, transform: &AffineMapTransform) {
        self.src_coord = transform.apply(self.src_coord);
        self.dest_coord = transform.apply(self.dest_coord);
        if self.calculated_coord != Coord::zero() {
            self.calculated_coord = transform.apply(self.calculated_coord);
        }
    }

    pub(crate) fn parse<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<Self> {
        let mut src = None;
        let mut dest = None;
        let mut calc = None;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(child) => match child.local_name().as_ref() {
                    b"source" => src = parse_inner_coord(reader).ok(),
                    b"destination" => dest = parse_inner_coord(reader).ok(),
                    b"calculated" => calc = parse_inner_coord(reader).ok(),
                    _ => {}
                },
                Event::End(be) => {
                    if be.local_name().as_ref() == b"passpoint" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::PassPoint));
                }
                _ => {}
            }
        }

        if let Some(src_coord) = src
            && let Some(dest_coord) = dest
            && let Some(calculated_coord) = calc
        {
            Ok(PassPoint {
                src_coord,
                dest_coord,
                calculated_coord,
            })
        } else {
            Err(Error::TemplateError)
        }
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
    fn multiply(&self, rhs: &Self) -> Self {
        let mut values = [0.; 9];
        for row in 0..3 {
            for col in 0..3 {
                values[row * 3 + col] = self.0[row * 3] * rhs.0[col]
                    + self.0[row * 3 + 1] * rhs.0[3 + col]
                    + self.0[row * 3 + 2] * rhs.0[6 + col];
            }
        }
        Self(values)
    }

    fn inverse_affine(&self) -> Option<Self> {
        let [a, b, tx, c, d, ty, _, _, _] = self.0;
        let det = a * d - b * c;
        if det == 0. {
            return None;
        }

        let inv_a = d / det;
        let inv_b = -b / det;
        let inv_c = -c / det;
        let inv_d = a / det;

        Some(Self([
            inv_a,
            inv_b,
            -(inv_a * tx + inv_b * ty),
            inv_c,
            inv_d,
            -(inv_c * tx + inv_d * ty),
            0.,
            0.,
            1.,
        ]))
    }

    pub(crate) fn parse<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<Self> {
        let mut values = [0.; 9];
        let mut i = 0;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(child) if child.local_name().as_ref() == b"element" => {
                    values[i] = try_get_attr_raw(&child, "value")?.unwrap_or(0.);
                    i += 1;
                }
                Event::End(be) if be.local_name().as_ref() == b"matrix" => break,
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::Matrix));
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
    let coord = to_file_coords(*coord)?;
    writer.write_event(Event::Start(BytesStart::new(wrapper_name)))?;
    writer.write_event(Event::Empty(BytesStart::new("coord").with_attributes([
        ("x", format!("{}", coord.x).as_str()),
        ("y", format!("{}", coord.y).as_str()),
    ])))?;
    writer.write_event(Event::End(BytesEnd::new(wrapper_name)))?;
    Ok(())
}

fn parse_inner_coord<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<Coord> {
    let mut coord = Coord::<i32>::zero();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(bs) if bs.local_name().as_ref() == b"coord" => {
                coord = Coord {
                    x: try_get_attr_raw(&bs, "x")?.unwrap_or(0),
                    y: try_get_attr_raw(&bs, "y")?.unwrap_or(0),
                };
            }
            Event::End(_) => break,
            Event::Eof => {
                return Err(Error::UnexpectedEof(OmapSection::CoordinateWrapper));
            }
            _ => {}
        }
    }
    Ok(from_file_coords(coord))
}
