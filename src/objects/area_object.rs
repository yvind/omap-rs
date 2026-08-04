use std::{cell::OnceCell, collections::HashMap};

use geo_types::{Coord, Polygon};
use linestring2bezier::{BezierSegment, BezierString};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{
    BezierPath, COORD_FLAGS_RING_END, FileCoord, bezier_from_raw_coords, file_coords_from_bezier,
};
use crate::{
    CoordinateComponent, Error, NonNegativeF64, OmapSection, Result,
    symbols::{Symbol, SymbolSet, WeakAreaPathSymbol},
    utils::{from_file_coords, to_file_coords, transform_position, try_get_attr_raw},
};

/// A polygon whose exterior and interior rings retain straight and cubic
/// Bézier segments.
#[derive(Debug, Clone)]
pub struct BezierPolygon {
    /// The polygon's exterior ring.
    pub exterior: BezierPath,
    /// The polygon's interior rings.
    pub interiors: Vec<BezierPath>,
}

/// A fill pattern rotation and origin used by area objects.
#[derive(Debug, Clone, Default)]
pub struct PatternRotation {
    /// Rotation of the fill pattern in radians.
    pub rotation: f64,
    /// Origin coordinate for the pattern.
    pub coord: Coord,
}

/// An area (polygon) object on the map.
#[derive(Debug, Clone)]
pub struct AreaObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    /// The fill-pattern rotation and origin.
    pub pattern_rotation: PatternRotation,
    /// The area or combined-area symbol used to render this object.
    pub symbol: WeakAreaPathSymbol,
    /// The permitted error when fitting Bézier curves for writing.
    ///
    /// Bézier fitting is enabled when this is [`Some`].
    pub bezier_write_error: Option<NonNegativeF64>,
    geometry: OnceCell<Polygon>,
    // store the raw map-file coords with flags so that the object can be written back unchanged if the coords are untouched
    // (so that the errors introduced when mapping from beziers to linestring and back only are introduced when necessary)
    raw_map_coords: Vec<FileCoord>,
    is_coords_touched: bool,
}

impl AreaObject {
    /// Create a new area object with the given symbol and geometry.
    pub fn new(symbol: impl Into<WeakAreaPathSymbol>, geometry: Polygon) -> Self {
        Self {
            tags: HashMap::new(),
            pattern_rotation: PatternRotation::default(),
            symbol: symbol.into(),
            bezier_write_error: None,
            geometry: OnceCell::from(geometry),
            raw_map_coords: Vec::new(),
            is_coords_touched: true,
        }
    }

    /// Consume this object and return its polygon geometry.
    ///
    /// # Errors
    ///
    /// Returns an error if the object has no usable geometry or if uncached raw
    /// geometry cannot be flattened with the requested error tolerance.
    pub fn into_geometry(self, allowed_error: f64) -> Result<Polygon> {
        if self.geometry_is_empty() {
            return Err(Error::ObjectError);
        }

        if self.geometry.get().is_none() {
            let geometry = self.flattened_geometry(allowed_error)?;
            self.geometry
                .set(geometry)
                .map_err(|_geometry| Error::ObjectError)?;
        }
        self.geometry.into_inner().ok_or(Error::ObjectError)
    }

    /// Get the polygon geometry, flattening and caching raw Bézier coordinates
    /// when needed.
    ///
    /// `allowed_error` is used only when initializing the cache. Later calls
    /// return the previously cached geometry.
    ///
    /// # Errors
    ///
    /// Returns an error if the object has no usable geometry or if its raw
    /// geometry cannot be flattened with the requested error tolerance.
    pub fn get_geometry(&self, allowed_error: f64) -> Result<&Polygon> {
        if self.geometry_is_empty() {
            return Err(Error::ObjectError);
        }

        if let Some(geometry) = self.geometry.get() {
            return Ok(geometry);
        }

        let geometry = self.flattened_geometry(allowed_error)?;
        self.geometry
            .set(geometry)
            .map_err(|_geometry| Error::ObjectError)?;
        self.geometry.get().ok_or(Error::ObjectError)
    }

    /// Rebuild the original area geometry as mixed straight/cubic Bézier
    /// rings, including dash-point metadata for every vertex.
    ///
    /// This is generated directly from the original file coordinates and
    /// therefore preserves the exact Bézier handles. Every returned ring is
    /// closed, matching [`Polygon`]'s invariant, and zero-segment rings are
    /// omitted. For a successfully parsed object, ring order and count match
    /// [`Self::get_geometry`].
    ///
    /// Returns [`None`] when the object was not read from raw file coordinates
    /// or after [`Self::get_geometry_mut`] has marked those coordinates as
    /// touched.
    pub fn bezier_geometry(&self) -> Option<BezierPolygon> {
        if self.is_coords_touched || self.raw_map_coords.is_empty() {
            return None;
        }

        bezier_polygon_from_raw_coords(&self.raw_map_coords)
    }

    /// Get a mutable reference to the polygon geometry, flattening it first
    /// when needed, and mark the coordinates as touched.
    ///
    /// `allowed_error` is used only when initializing the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the object has no usable geometry or if its raw
    /// geometry cannot be flattened with the requested error tolerance.
    pub fn get_geometry_mut(&mut self, allowed_error: f64) -> Result<&mut Polygon> {
        if self.geometry_is_empty() {
            return Err(Error::ObjectError);
        }

        if self.geometry.get().is_none() {
            let geometry = self.flattened_geometry(allowed_error)?;
            self.geometry
                .set(geometry)
                .map_err(|_geometry| Error::ObjectError)?;
        }
        self.is_coords_touched = true;
        self.geometry.get_mut().ok_or(Error::ObjectError)
    }

    /// Iterate over the raw file coordinates in mm with their flags.
    ///
    /// These are the original control points (including Bézier handles) as read
    /// from the file, converted from µm integers to mm floats. See the
    /// `COORD_FLAG_*` constants in this module for the flag assignments.
    ///
    /// The iterator is empty for objects not read from file data.
    pub fn raw_coords(&self) -> impl ExactSizeIterator<Item = (Coord, u8)> + '_ {
        self.raw_map_coords
            .iter()
            .map(|(c, flag)| (from_file_coords(*c), *flag))
    }

    pub(crate) fn geometry_is_empty(&self) -> bool {
        self.geometry
            .get()
            .map_or(self.raw_map_coords.len() < 2, |geometry| {
                geometry.exterior().0.len() < 2
            })
    }

    /// Apply a coordinate transform to both the geometry and the raw
    /// control points, preserving Bézier structure without re-approximation.
    ///
    /// This does **not** mark the coordinates as touched, so the raw (transformed) control
    /// points (with Bézier flags) will still be used on write.
    ///
    /// # Errors
    ///
    /// Returns any error produced by `transform`, or an error if a transformed
    /// raw coordinate is outside the file-format range.
    pub fn apply_transform<F>(&mut self, transform: &F) -> Result<()>
    where
        F: Fn(Coord) -> Result<Coord> + ?Sized,
    {
        // Transform the discretized geometry if it has been initialized.
        if let Some(geometry) = self.geometry.get_mut() {
            let mut error = None;
            geometry.exterior_mut(|ext| {
                for coord in &mut ext.0 {
                    match transform(*coord) {
                        Ok(transformed) => *coord = transformed,
                        Err(err) => {
                            error = Some(err);
                            break;
                        }
                    }
                }
            });
            if let Some(error) = error {
                return Err(error);
            }
            geometry.interiors_mut(|interiors| {
                for interior in interiors.iter_mut() {
                    for coord in &mut interior.0 {
                        match transform(*coord) {
                            Ok(transformed) => *coord = transformed,
                            Err(err) => {
                                error = Some(err);
                                return;
                            }
                        }
                    }
                }
            });
            if let Some(error) = error {
                return Err(error);
            }
        }
        // Transform the pattern rotation origin and its local orientation.
        let (pattern_coord, pattern_rotation, _) =
            transform_position(self.pattern_rotation.coord, transform)?;
        self.pattern_rotation.coord = pattern_coord;
        self.pattern_rotation.rotation += pattern_rotation;
        // Transform raw control points — flags stay unchanged
        for (file_coord, _flag) in &mut self.raw_map_coords {
            let map_coord = from_file_coords(*file_coord);
            *file_coord = to_file_coords(transform(map_coord)?)?;
        }
        // Do NOT set is_coords_touched = true
        Ok(())
    }

    /// Reverse the winding order of all rings.
    pub fn reverse_polygon(&mut self) {
        if let Some(geometry) = self.geometry.get_mut() {
            geometry.exterior_mut(|e| e.0.reverse());
            geometry.interiors_mut(|is| is.iter_mut().for_each(|i| i.0.reverse()));
        }

        self.raw_map_coords = reverse_raw_polygon_coords(&self.raw_map_coords);
    }

    /// Create an `AreaObject` for use as a `PointSymbol` element (no map symbol needed)
    pub fn new_element(geometry: Polygon) -> Self {
        Self {
            tags: HashMap::new(),
            pattern_rotation: PatternRotation::default(),
            symbol: WeakAreaPathSymbol::Area(std::rc::Weak::new()),
            bezier_write_error: None,
            geometry: OnceCell::from(geometry),
            raw_map_coords: Vec::new(),
            is_coords_touched: true,
        }
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let idx = match &self.symbol {
            WeakAreaPathSymbol::Area(weak) => {
                if let Some(sym) = weak.upgrade() {
                    symbol_set
                        .iter()
                        .position(|s| match s {
                            Symbol::Area(ref_cell) => ref_cell.as_ptr() == sym.as_ptr(),
                            _ => false,
                        })
                        .map(|p| p as i32)
                        .unwrap_or(-1)
                } else {
                    -1
                }
            }
            WeakAreaPathSymbol::CombinedArea(weak) => {
                if let Some(sym) = weak.upgrade() {
                    symbol_set
                        .iter()
                        .position(|s| match s {
                            Symbol::CombinedArea(ref_cell) => ref_cell.as_ptr() == sym.as_ptr(),
                            _ => false,
                        })
                        .map(|p| p as i32)
                        .unwrap_or(-1)
                } else {
                    -1
                }
            }
        };
        self.write_content(writer, Some(idx))?;
        Ok(())
    }

    /// Write a full `<object>...</object>` element - used for point symbol elements
    pub(crate) fn write_as_element<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        self.write_content(writer, None)?;
        Ok(())
    }

    /// Write the object.
    /// Uses raw coords if untouched, otherwise writes geometry.
    fn write_content<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_index: Option<i32>,
    ) -> Result<()> {
        if self.geometry_is_empty() {
            return Ok(());
        }

        let mut bs = BytesStart::new("object").with_attributes([("type", "1")]);
        if let Some(sid) = symbol_index {
            bs.push_attribute(("symbol", sid.to_string().as_str()));
        }
        writer.write_event(Event::Start(bs))?;
        // elements are not allowed to have tags
        if !self.tags.is_empty() && symbol_index.is_some() {
            super::write_tags(writer, &self.tags)?;
        }

        if !self.is_coords_touched {
            super::write_raw_coords(writer, &self.raw_map_coords)?;
        } else {
            let geometry = self.geometry.get().ok_or(Error::ObjectError)?;
            self.write_geometry_coords(writer, geometry)?;
        }
        self.write_pattern(writer)?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Write coords from the geometry, fitting Béziers when requested.
    fn write_geometry_coords<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        geometry: &Polygon,
    ) -> Result<()> {
        let mut all_coords: Vec<FileCoord> = Vec::new();

        if let Some(bezier_error) = self.bezier_write_error {
            for ring in std::iter::once(geometry.exterior()).chain(geometry.interiors()) {
                let bezier = BezierString::from_line_string(ring.clone(), bezier_error.get())?;
                all_coords.extend(file_coords_from_bezier(&bezier, COORD_FLAGS_RING_END)?);
            }
        } else {
            for ring in std::iter::once(geometry.exterior()).chain(geometry.interiors()) {
                for (index, coord) in ring.coords().enumerate() {
                    let flag = if index + 1 == ring.0.len() {
                        COORD_FLAGS_RING_END
                    } else {
                        0
                    };
                    all_coords.push((to_file_coords(*coord)?, flag));
                }
            }
        }

        super::write_raw_coords(writer, &all_coords)?;
        Ok(())
    }

    /// Write the `<pattern>` element with the pattern rotation and origin coord
    fn write_pattern<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let pr = &self.pattern_rotation;
        let mut bs = BytesStart::new("pattern");
        bs.push_attribute(("rotation", pr.rotation.to_string().as_str()));
        writer.write_event(Event::Start(bs))?;
        let fc = to_file_coords(pr.coord)?;
        writer.write_event(Event::Empty(BytesStart::new("coord").with_attributes([
            ("x", fc.x.to_string().as_str()),
            ("y", fc.y.to_string().as_str()),
        ])))?;
        writer.write_event(Event::End(BytesEnd::new("pattern")))?;
        Ok(())
    }

    /// Parse an area object. The reader should be positioned right after the `<object>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: WeakAreaPathSymbol,
    ) -> Result<Self> {
        let mut tags = HashMap::new();
        let mut pr = PatternRotation::default();
        let mut raw_map_coords = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"coords" => {
                        let num_coords: usize = try_get_attr_raw(&bytes_start, "count")
                            .ok()
                            .flatten()
                            .unwrap_or(0);
                        raw_map_coords.reserve(num_coords);
                    }
                    b"pattern" => {
                        pr.rotation = try_get_attr_raw(&bytes_start, "rotation")
                            .ok()
                            .flatten()
                            .unwrap_or(pr.rotation);
                    }
                    b"tags" => tags = super::parse_tags(reader)?,
                    b"coord" => {
                        let x = try_get_attr_raw(&bytes_start, "x")?.unwrap_or(0);
                        let y = try_get_attr_raw(&bytes_start, "y")?.unwrap_or(0);
                        pr.coord = from_file_coords(Coord { x, y });
                    }
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Text(bytes_text) => {
                    let raw_xml = str::from_utf8(bytes_text.as_ref())?;

                    for vertex in raw_xml.split_terminator(';') {
                        let mut parts: (i32, i32, u8) = (0, 0, 0);
                        let mut split = vertex.split_whitespace();

                        parts.0 = split
                            .next()
                            .ok_or(Error::MissingCoordinateComponent(CoordinateComponent::X))?
                            .parse()?;
                        parts.1 = split
                            .next()
                            .ok_or(Error::MissingCoordinateComponent(CoordinateComponent::Y))?
                            .parse()?;
                        if let Some(e) = split.next() {
                            parts.2 = e.parse()?;
                        }

                        raw_map_coords.push((
                            Coord {
                                x: parts.0,
                                y: parts.1,
                            },
                            parts.2,
                        ));
                    }
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::AreaObject));
                }
                _ => (),
            }
        }
        Ok(Self {
            tags,
            pattern_rotation: pr,
            symbol,
            bezier_write_error: None,
            geometry: OnceCell::new(),
            raw_map_coords,
            is_coords_touched: false,
        })
    }

    fn flattened_geometry(&self, allowed_error: f64) -> Result<Polygon> {
        let bezier =
            bezier_polygon_from_raw_coords(&self.raw_map_coords).ok_or(Error::ObjectError)?;
        let exterior = bezier.exterior.geometry().to_line_string(allowed_error)?;
        let interiors = bezier
            .interiors
            .iter()
            .map(|ring| ring.geometry().to_line_string(allowed_error))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Polygon::new(exterior, interiors))
    }
}

fn bezier_polygon_from_raw_coords(coords: &[FileCoord]) -> Option<BezierPolygon> {
    let mut rings = Vec::new();
    let mut ring_start = 0;

    for (index, (_, flag)) in coords.iter().enumerate() {
        if flag & COORD_FLAGS_RING_END != 0 {
            if let Some(mut ring) = bezier_from_raw_coords(&coords[ring_start..=index]) {
                close_bezier_ring(&mut ring);
                rings.push(ring);
            }
            ring_start = index + 1;
        }
    }

    // Mapper normally terminates every ring with a close/hole flag, but
    // tolerate an implicitly closed final ring, as parse does.
    if ring_start < coords.len()
        && let Some(mut ring) = bezier_from_raw_coords(&coords[ring_start..])
    {
        close_bezier_ring(&mut ring);
        rings.push(ring);
    }

    let mut rings = rings.into_iter();
    Some(BezierPolygon {
        exterior: rings.next()?,
        interiors: rings.collect(),
    })
}

fn close_bezier_ring(ring: &mut BezierPath) {
    let Some(first) = ring.geometry.segments().next().map(BezierSegment::start) else {
        return;
    };
    let Some(last) = ring.geometry.segments().last().map(BezierSegment::end) else {
        return;
    };

    if last == first {
        let seam_is_dash_point = ring.vertex_is_dash_point.first().copied().unwrap_or(false)
            || ring.vertex_is_dash_point.last().copied().unwrap_or(false);
        if let Some(first_is_dash_point) = ring.vertex_is_dash_point.first_mut() {
            *first_is_dash_point = seam_is_dash_point;
        }
        if let Some(last_is_dash_point) = ring.vertex_is_dash_point.last_mut() {
            *last_is_dash_point = seam_is_dash_point;
        }
        return;
    }

    ring.geometry.0.push(BezierSegment::new(last, None, first));
    ring.vertex_is_dash_point
        .push(ring.vertex_is_dash_point.first().copied().unwrap_or(false));
}

pub(crate) fn reverse_raw_polygon_coords(coords: &[FileCoord]) -> Vec<FileCoord> {
    // get each of the substrings for each loop and flip them
    // a substring ends with a 2 flag (often 18 or 50)
    let mut s = Vec::with_capacity(coords.len());
    let mut prev_split = 0;
    for (i, (_, f)) in coords.iter().enumerate() {
        if f & COORD_FLAGS_RING_END != 0 || i == coords.len() - 1 {
            s.extend(crate::objects::line_object::reverse_raw_line_coords(
                &coords[prev_split..=i],
            ));
            prev_split = i + 1;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use geo_types::Coord;
    use quick_xml::{Reader, Writer, events::Event};

    use super::{AreaObject, bezier_polygon_from_raw_coords};
    use crate::objects::{COORD_FLAG_CLOSE_POINT, COORD_FLAG_DASH_POINT, COORD_FLAG_HOLE_POINT};
    use crate::{Error, Result, symbols::WeakAreaPathSymbol};

    #[test]
    fn every_bezier_polygon_ring_is_closed() {
        let polygon = bezier_polygon_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, COORD_FLAG_DASH_POINT),
            (Coord { x: 2_000, y: 0 }, 0),
            (Coord { x: 1_000, y: 1_000 }, COORD_FLAG_CLOSE_POINT),
            (Coord { x: 500, y: 250 }, 0),
            (Coord { x: 1_000, y: 250 }, 0),
            (
                Coord { x: 750, y: 750 },
                COORD_FLAG_CLOSE_POINT | COORD_FLAG_HOLE_POINT,
            ),
        ]);
        assert!(polygon.is_some());
        let Some(polygon) = polygon else {
            return;
        };

        assert_eq!(polygon.interiors.len(), 1);
        for ring in std::iter::once(&polygon.exterior).chain(&polygon.interiors) {
            assert!(!ring.geometry.0.is_empty());
            let first = ring.geometry.0[0].start();
            let last = ring.geometry.0[ring.geometry.0.len() - 1].end();
            assert_eq!(first, last);
            assert_eq!(
                ring.vertex_is_dash_point.len(),
                ring.geometry.num_segments() + 1
            );
            assert_eq!(
                ring.vertex_is_dash_point.first(),
                ring.vertex_is_dash_point.last()
            );
        }
        assert_eq!(
            polygon.exterior.vertex_is_dash_point,
            [true, false, false, true]
        );
    }

    #[test]
    fn zero_segment_rings_are_omitted() {
        let polygon = bezier_polygon_from_raw_coords(&[
            (Coord { x: 0, y: 0 }, 0),
            (Coord { x: 1_000, y: 0 }, 0),
            (Coord { x: 0, y: 0 }, COORD_FLAG_CLOSE_POINT),
            (
                Coord { x: 500, y: 500 },
                COORD_FLAG_CLOSE_POINT | COORD_FLAG_HOLE_POINT,
            ),
        ]);
        assert!(polygon.is_some());
        let Some(polygon) = polygon else {
            return;
        };

        assert!(polygon.interiors.is_empty());
    }

    #[test]
    fn parsed_polygon_accessors_have_matching_closed_rings() -> Result<()> {
        let mut reader = Reader::from_str(
            r#"<object><coords count="6">0 0 32;2000 0;1000 1000 2;500 250;1000 250;750 750 18;</coords><pattern rotation="0"></pattern></object>"#,
        );
        let event = reader.read_event()?;
        assert!(matches!(event, Event::Start(_)));

        let mut area =
            AreaObject::parse(&mut reader, WeakAreaPathSymbol::Area(std::rc::Weak::new()))?;
        let bezier = area.bezier_geometry().ok_or(Error::ObjectError)?;

        assert!(area.geometry.get().is_none());
        assert!(!area.is_coords_touched);
        let mut writer = Writer::new(Vec::new());
        area.write_content(&mut writer, None)?;
        assert!(area.geometry.get().is_none());
        let output = String::from_utf8(writer.into_inner())?;
        assert!(output.contains("0 0 32;2000 0;1000 1000 2;"));

        assert_eq!(area.get_geometry(0.1)?.interiors().len(), 1);
        assert!(area.geometry.get().is_some());
        assert!(!area.is_coords_touched);
        assert_eq!(bezier.interiors.len(), 1);
        assert_eq!(area.raw_coords().len(), 6);
        assert_eq!(
            bezier.exterior.vertex_is_dash_point,
            [true, false, false, true]
        );
        assert_eq!(
            bezier.exterior.vertex_is_dash_point.first(),
            bezier.exterior.vertex_is_dash_point.last()
        );
        let _ = area.get_geometry_mut(0.2)?;
        assert!(area.is_coords_touched);
        assert!(area.bezier_geometry().is_none());
        Ok(())
    }
}
