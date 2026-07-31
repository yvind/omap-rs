use std::{cell::OnceCell, collections::HashMap};

use geo_types::{Coord, Polygon};
use linestring2bezier::{BezierSegment, BezierString};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{
    BezierPath, COORD_FLAGS_RING_END, FileCoord, bezier_from_raw_coords, file_coords_from_bezier,
    straight_bezier_from_line_string,
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
    // the exact rings rebuilt from raw_map_coords, dropped whenever the coords change
    bezier: OnceCell<BezierPolygon>,
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
            bezier: OnceCell::new(),
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

    /// The original area geometry as mixed straight/cubic Bézier rings,
    /// including dash-point metadata for every vertex.
    ///
    /// This is generated directly from the original file coordinates and
    /// therefore preserves the exact Bézier handles. Every returned ring is
    /// closed, matching [`Polygon`]'s invariant, and zero-segment rings are
    /// omitted. For a successfully parsed object, ring order and count match
    /// [`Self::get_geometry`].
    ///
    /// The rings are built on the first call and cached, like the flattened
    /// geometry behind [`Self::get_geometry`]; the cache is dropped whenever
    /// the coordinates change.
    ///
    /// Returns [`None`] when the object was not read from raw file coordinates
    /// or after [`Self::get_geometry_mut`] has marked those coordinates as
    /// touched.
    pub fn bezier_geometry(&self) -> Option<&BezierPolygon> {
        if self.is_coords_touched || self.raw_map_coords.is_empty() {
            return None;
        }

        if let Some(bezier) = self.bezier.get() {
            return Some(bezier);
        }

        let bezier = bezier_polygon_from_raw_coords(&self.raw_map_coords)?;
        let _ = self.bezier.set(bezier);
        self.bezier.get()
    }

    /// The area geometry as mixed straight/cubic Bézier rings, whether or not
    /// the original control points survived.
    ///
    /// This is [`Self::bezier_geometry`] wherever that yields rings. For an
    /// object built in memory, or one whose coordinates have been marked as
    /// touched by [`Self::get_geometry_mut`], the flattened [`Polygon`] is
    /// lifted back into rings of straight segments instead, with no vertex
    /// flagged as a dash point — the only honest answer once the raw flags are
    /// gone.
    ///
    /// Consumers that want the geometry, and would treat missing Bézier rings
    /// as all-straight anyway, want this one; reach for
    /// [`Self::bezier_geometry`] when the difference matters. The rings are
    /// cached the same way.
    ///
    /// # Errors
    ///
    /// Returns an error if the object has no usable geometry.
    pub fn bezier_geometry_or_straight(&self) -> Result<&BezierPolygon> {
        if self.geometry_is_empty() {
            return Err(Error::ObjectError);
        }

        if let Some(bezier) = self.bezier.get() {
            return Ok(bezier);
        }

        let bezier = if self.is_coords_touched {
            straight_bezier_polygon(self.geometry.get().ok_or(Error::ObjectError)?)
        } else {
            bezier_polygon_from_raw_coords(&self.raw_map_coords)
        }
        .ok_or(Error::ObjectError)?;

        let _ = self.bezier.set(bezier);
        self.bezier.get().ok_or(Error::ObjectError)
    }

    /// Rebuild the original area geometry as mixed straight/cubic Bézier
    /// rings.
    ///
    /// Prefer [`Self::bezier_geometry`], which hands out the cached rings
    /// instead of a fresh clone.
    #[deprecated(note = "renamed to bezier_geometry")]
    pub fn get_geometry_bezier(&self) -> Option<BezierPolygon> {
        self.bezier_geometry().cloned()
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
        // The caller may edit the geometry, so the cached rings are now stale.
        self.bezier = OnceCell::new();
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

    /// Get the raw file coordinates in mm with their flags.
    ///
    /// Prefer [`Self::raw_coords`] when a collected [`Vec`] is not needed.
    #[deprecated(note = "use the allocation-free raw_coords iterator")]
    pub fn get_raw_coords(&self) -> Vec<(Coord, u8)> {
        self.raw_coords().collect()
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
        F: Fn(geo_types::Coord) -> Result<geo_types::Coord> + ?Sized,
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
        // The raw coords moved, so the cached rings are stale — they are
        // rebuilt from the transformed coords on the next request.
        self.bezier = OnceCell::new();
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
        self.bezier = OnceCell::new();
    }

    /// Create an `AreaObject` for use as a `PointSymbol` element (no map symbol needed)
    pub fn new_element(geometry: Polygon) -> Self {
        Self {
            tags: HashMap::new(),
            pattern_rotation: PatternRotation::default(),
            symbol: WeakAreaPathSymbol::Area(std::rc::Weak::new()),
            bezier_write_error: None,
            geometry: OnceCell::from(geometry),
            bezier: OnceCell::new(),
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
            bezier: OnceCell::new(),
            raw_map_coords,
            is_coords_touched: false,
        })
    }

    fn flattened_geometry(&self, allowed_error: f64) -> Result<Polygon> {
        // Flatten the cached rings when there already are some — same result,
        // one reconstruction less. Not populated here: a consumer that only
        // ever asks for the flattened geometry should not pay to keep them.
        if let Some(bezier) = self.bezier.get() {
            return flatten_bezier_polygon(bezier, allowed_error);
        }

        let bezier =
            bezier_polygon_from_raw_coords(&self.raw_map_coords).ok_or(Error::ObjectError)?;
        flatten_bezier_polygon(&bezier, allowed_error)
    }
}

/// Lift an already flattened polygon back into Bézier form, one straight ring
/// per [`Polygon`] ring. Zero-segment rings are omitted, as they are when the
/// rings are rebuilt from raw coordinates.
fn straight_bezier_polygon(polygon: &Polygon) -> Option<BezierPolygon> {
    Some(BezierPolygon {
        exterior: straight_bezier_from_line_string(polygon.exterior())?,
        interiors: polygon
            .interiors()
            .iter()
            .filter_map(straight_bezier_from_line_string)
            .collect(),
    })
}

fn flatten_bezier_polygon(bezier: &BezierPolygon, allowed_error: f64) -> Result<Polygon> {
    let exterior = bezier.exterior.geometry().to_line_string(allowed_error)?;
    let interiors = bezier
        .interiors
        .iter()
        .map(|ring| ring.geometry().to_line_string(allowed_error))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Polygon::new(exterior, interiors))
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
    use geo_types::{Coord, LineString, Polygon};
    use linestring2bezier::BezierSegment;
    use quick_xml::{Reader, Writer, events::Event};

    use super::{AreaObject, bezier_polygon_from_raw_coords};
    use crate::geo_referencing::{AffineMapTransform, GeoRef, MapTransform};
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

    const RING_XML: &str = r#"<object><coords count="6">0 0 32;2000 0;1000 1000 2;500 250;1000 250;750 750 18;</coords><pattern rotation="0"></pattern></object>"#;

    fn parse_area(xml: &str) -> Result<AreaObject> {
        let mut reader = Reader::from_str(xml);
        let event = reader.read_event()?;
        assert!(matches!(event, Event::Start(_)), "expected an object start");

        AreaObject::parse(&mut reader, WeakAreaPathSymbol::Area(std::rc::Weak::new()))
    }

    /// An affine transform that only translates, built the only way the public
    /// API allows: as the difference between two map reference points.
    fn translation(dx: f64, dy: f64) -> Result<AffineMapTransform> {
        let before = GeoRef::new(10_000).get_transform();
        let mut after = GeoRef::new(10_000);
        after.map_ref_point = Coord { x: dx, y: dy };
        MapTransform::affine_between(&before, &after.get_transform())
    }

    fn exterior_start(area: &AreaObject) -> Result<geo_types::Coord> {
        area.bezier_geometry()
            .ok_or(Error::ObjectError)?
            .exterior
            .geometry()
            .segments()
            .next()
            .map(BezierSegment::start)
            .ok_or(Error::ObjectError)
    }

    #[test]
    fn bezier_geometry_is_cached_until_the_coords_change() -> Result<()> {
        let mut area = parse_area(RING_XML)?;

        let first = std::ptr::from_ref(area.bezier_geometry().ok_or(Error::ObjectError)?);
        let second = std::ptr::from_ref(area.bezier_geometry().ok_or(Error::ObjectError)?);
        assert!(
            std::ptr::eq(first, second),
            "a second call must hand out the cached rings, not rebuilt ones"
        );

        // Both of these move the raw coords without marking them as touched.
        area.apply_affine(&translation(10., 20.)?);
        assert_eq!(
            exterior_start(&area)?,
            Coord { x: 10., y: 20. },
            "the cache must not survive apply_affine"
        );

        area.reverse_polygon();
        assert_eq!(
            exterior_start(&area)?,
            Coord { x: 11., y: 19. },
            "the cache must not survive reverse_polygon"
        );

        // Touching the coords drops the rings for good.
        let _ = area.get_geometry_mut(0.1)?;
        assert!(area.bezier_geometry().is_none());
        Ok(())
    }

    #[test]
    fn bezier_geometry_or_straight_keeps_the_exact_rings() -> Result<()> {
        let area = parse_area(RING_XML)?;

        let exact = std::ptr::from_ref(area.bezier_geometry().ok_or(Error::ObjectError)?);
        let always = std::ptr::from_ref(area.bezier_geometry_or_straight()?);
        assert!(
            std::ptr::eq(exact, always),
            "must not fall back while the raw coords are intact"
        );
        assert_eq!(
            area.bezier_geometry_or_straight()?
                .exterior
                .vertex_is_dash_point(),
            [true, false, false, true],
            "the dash flags must survive"
        );
        Ok(())
    }

    #[test]
    fn bezier_geometry_or_straight_falls_back_to_straight_rings() -> Result<()> {
        let exterior = LineString::new(vec![
            Coord { x: 0., y: 0. },
            Coord { x: 4., y: 0. },
            Coord { x: 4., y: 4. },
        ]);
        let interior = LineString::new(vec![
            Coord { x: 1., y: 1. },
            Coord { x: 2., y: 1. },
            Coord { x: 2., y: 2. },
        ]);
        let area = AreaObject::new(
            WeakAreaPathSymbol::Area(std::rc::Weak::new()),
            Polygon::new(exterior, vec![interior]),
        );

        assert!(
            area.bezier_geometry().is_none(),
            "an object built in memory has no raw coords"
        );

        let bezier = area.bezier_geometry_or_straight()?;
        assert_eq!(bezier.interiors.len(), 1);
        for ring in std::iter::once(&bezier.exterior).chain(&bezier.interiors) {
            // geo_types closes the rings, so each has three segments.
            assert_eq!(ring.geometry().num_segments(), 3);
            assert!(
                ring.geometry()
                    .segments()
                    .all(|segment| matches!(segment, BezierSegment::Line(_))),
                "no handles can be invented"
            );
            assert_eq!(ring.vertex_is_dash_point(), [false; 4]);
            assert_eq!(
                ring.geometry().segments().next().map(BezierSegment::start),
                ring.geometry().segments().last().map(BezierSegment::end),
                "every ring must stay closed"
            );
        }
        Ok(())
    }

    #[test]
    fn bezier_geometry_or_straight_follows_the_edited_geometry() -> Result<()> {
        let mut area = parse_area(RING_XML)?;
        // Cache the exact rings first, so handing out stale ones would show.
        assert!(area.bezier_geometry().is_some());

        *area.get_geometry_mut(0.01)? = Polygon::new(
            LineString::new(vec![
                Coord { x: 0., y: 0. },
                Coord { x: 2., y: 0. },
                Coord { x: 2., y: 2. },
            ]),
            Vec::new(),
        );

        assert!(area.bezier_geometry().is_none());
        let bezier = area.bezier_geometry_or_straight()?;
        assert!(bezier.interiors.is_empty(), "the hole is gone");
        assert_eq!(bezier.exterior.geometry().num_segments(), 3);
        assert!(
            bezier
                .exterior
                .vertex_is_dash_point()
                .iter()
                .all(|is_dash_point| !is_dash_point),
            "the dash flags do not survive an edit"
        );
        Ok(())
    }

    #[test]
    fn flattening_is_unaffected_by_cached_rings() -> Result<()> {
        let cached = parse_area(RING_XML)?;
        assert!(cached.bezier_geometry().is_some(), "expected cached rings");
        let plain = parse_area(RING_XML)?;

        assert_eq!(cached.get_geometry(0.01)?, plain.get_geometry(0.01)?);
        Ok(())
    }

    #[test]
    fn parsed_polygon_accessors_have_matching_closed_rings() -> Result<()> {
        let mut area = parse_area(RING_XML)?;
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
