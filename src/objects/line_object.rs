use std::{cell::OnceCell, collections::HashMap};

use geo_types::{Coord, LineString};
use linestring2bezier::BezierString;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{
    BezierPath, COORD_FLAG_CURVE_START, COORD_FLAGS_RING_END, FileCoord, bezier_from_raw_coords,
    file_coords_from_bezier, straight_bezier_from_line_string,
};
use crate::{
    CoordinateComponent, Error, NonNegativeF64, OmapSection, Result,
    symbols::{Symbol, SymbolSet, WeakLinePathSymbol},
    utils::{from_file_coords, to_file_coords, try_get_attr_raw},
};

/// A line object represented as a polyline on the map.
#[derive(Debug, Clone)]
pub struct LineObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    /// The line or combined-line symbol used to render this object.
    pub symbol: WeakLinePathSymbol,
    /// The permitted error when fitting Bézier curves for writing.
    ///
    /// Bézier fitting is enabled when this is [`Some`].
    pub bezier_write_error: Option<NonNegativeF64>,
    geometry: OnceCell<LineString>,
    // the exact path rebuilt from raw_map_coords, dropped whenever the coords change
    bezier: OnceCell<BezierPath>,
    // store the raw map-file coords with flags so that the object can be written back unchanged if the coords are untouched
    // (so that the errors introduced when mapping from beziers to linestring and back only are introduced when necessary)
    raw_map_coords: Vec<FileCoord>,
    is_coords_touched: bool,
}

impl LineObject {
    /// Create a new line object with the given symbol and geometry.
    pub fn new(symbol: impl Into<WeakLinePathSymbol>, geometry: LineString) -> Self {
        Self {
            tags: HashMap::new(),
            symbol: symbol.into(),
            bezier_write_error: None,
            geometry: OnceCell::from(geometry),
            bezier: OnceCell::new(),
            raw_map_coords: Vec::new(),
            is_coords_touched: true,
        }
    }

    /// Get the line geometry, flattening and caching raw Bézier coordinates
    /// when needed.
    ///
    /// `allowed_error` is used only when initializing the cache. Later calls
    /// return the previously cached geometry.
    ///
    /// # Errors
    ///
    /// Returns an error if the object has no usable geometry or if its raw
    /// geometry cannot be flattened with the requested error tolerance.
    pub fn get_geometry(&self, allowed_error: f64) -> Result<&LineString> {
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

    /// The original line geometry as a mixed straight/cubic Bézier path,
    /// including dash-point metadata for every vertex.
    ///
    /// This is generated directly from the original file coordinates and
    /// therefore preserves the exact Bézier handles. The path is built on the
    /// first call and cached, like the flattened geometry behind
    /// [`Self::get_geometry`]; the cache is dropped whenever the coordinates
    /// change.
    ///
    /// Returns [`None`] when the object was not read from raw file coordinates,
    /// the coordinates do not form a segment, or [`Self::get_geometry_mut`] has
    /// marked them as touched.
    pub fn bezier_geometry(&self) -> Option<&BezierPath> {
        if self.is_coords_touched {
            return None;
        }

        if let Some(bezier) = self.bezier.get() {
            return Some(bezier);
        }

        let bezier = bezier_from_raw_coords(&self.raw_map_coords)?;
        let _ = self.bezier.set(bezier);
        self.bezier.get()
    }

    /// The line geometry as a mixed straight/cubic Bézier path, whether or not
    /// the original control points survived.
    ///
    /// This is [`Self::bezier_geometry`] wherever that yields a path. For an
    /// object built in memory, or one whose coordinates have been marked as
    /// touched by [`Self::get_geometry_mut`], the flattened [`LineString`] is
    /// lifted back into a path of straight segments instead, with no vertex
    /// flagged as a dash point — the only honest answer once the raw flags are
    /// gone.
    ///
    /// Consumers that want the geometry, and would treat a missing Bézier form
    /// as all-straight anyway, want this one; reach for
    /// [`Self::bezier_geometry`] when the difference matters. The path is
    /// cached the same way.
    ///
    /// # Errors
    ///
    /// Returns an error if the object has no usable geometry.
    pub fn bezier_geometry_or_straight(&self) -> Result<&BezierPath> {
        if self.geometry_is_empty() {
            return Err(Error::ObjectError);
        }

        if let Some(bezier) = self.bezier.get() {
            return Ok(bezier);
        }

        let bezier = if self.is_coords_touched {
            straight_bezier_from_line_string(self.geometry.get().ok_or(Error::ObjectError)?)
        } else {
            bezier_from_raw_coords(&self.raw_map_coords)
        }
        .ok_or(Error::ObjectError)?;

        let _ = self.bezier.set(bezier);
        self.bezier.get().ok_or(Error::ObjectError)
    }

    /// Rebuild the original line geometry as a mixed straight/cubic Bézier
    /// path.
    ///
    /// Prefer [`Self::bezier_geometry`], which hands out the cached path
    /// instead of a fresh clone.
    #[deprecated(note = "renamed to bezier_geometry")]
    pub fn get_geometry_bezier(&self) -> Option<BezierPath> {
        self.bezier_geometry().cloned()
    }

    /// Get a mutable reference to the line geometry, flattening it first when
    /// needed, and mark the coordinates as touched.
    ///
    /// `allowed_error` is used only when initializing the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the object has no usable geometry or if its raw
    /// geometry cannot be flattened with the requested error tolerance.
    pub fn get_geometry_mut(&mut self, allowed_error: f64) -> Result<&mut LineString> {
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
        // The caller may edit the geometry, so the cached path is now stale.
        self.bezier = OnceCell::new();
        self.geometry.get_mut().ok_or(Error::ObjectError)
    }

    /// Consume this object and return its line geometry.
    ///
    /// # Errors
    ///
    /// Returns an error if the object has no usable geometry or if uncached raw
    /// geometry cannot be flattened with the requested error tolerance.
    pub fn into_geometry(self, allowed_error: f64) -> Result<LineString> {
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

    /// Create a `LineObject` for use as a `PointSymbol` element (no map symbol needed)
    pub fn new_element(geometry: LineString) -> Self {
        Self {
            tags: HashMap::new(),
            symbol: WeakLinePathSymbol::Line(std::rc::Weak::new()),
            bezier_write_error: None,
            geometry: OnceCell::from(geometry),
            bezier: OnceCell::new(),
            raw_map_coords: Vec::new(),
            is_coords_touched: true,
        }
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
                geometry.0.len() < 2
            })
    }

    /// Apply a coordinate transform to both the geometry and the raw
    /// control points, preserving Bézier structure without re-approximation.
    ///
    /// This does **not** mark the coordinates as touched, so the raw transformed control
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
            for coord in &mut geometry.0 {
                *coord = transform(*coord)?;
            }
        }
        // Transform raw control points — flags stay unchanged
        for (file_coord, _flag) in &mut self.raw_map_coords {
            let map_coord = from_file_coords(*file_coord);
            *file_coord = to_file_coords(transform(map_coord)?)?;
        }
        // The raw coords moved, so the cached path is stale — it is rebuilt
        // from the transformed coords on the next request.
        self.bezier = OnceCell::new();
        // Do NOT set is_coords_touched = true
        Ok(())
    }

    /// Reverses a geometry and the input xml without marking it as touched
    pub fn reverse_linestring(&mut self) {
        if let Some(geometry) = self.geometry.get_mut() {
            geometry.0.reverse();
        }
        self.raw_map_coords = reverse_raw_line_coords(&self.raw_map_coords);
        self.bezier = OnceCell::new();
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let idx = match &self.symbol {
            WeakLinePathSymbol::Line(weak) => {
                if let Some(sym) = weak.upgrade() {
                    symbol_set
                        .iter()
                        .position(|s| match s {
                            Symbol::Line(ref_cell) => ref_cell.as_ptr() == sym.as_ptr(),
                            _ => false,
                        })
                        .map(|p| p as i32)
                        .unwrap_or(-1)
                } else {
                    -1
                }
            }
            WeakLinePathSymbol::CombinedLine(weak) => {
                if let Some(sym) = weak.upgrade() {
                    symbol_set
                        .iter()
                        .position(|s| match s {
                            Symbol::CombinedLine(ref_cell) => ref_cell.as_ptr() == sym.as_ptr(),
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

    /// Write the object
    /// Uses raw coords if untouched, otherwise writes geometry
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
            if geometry.0.len() < 2 {
                return Err(Error::ObjectError);
            }
            self.write_geometry_coords(writer, geometry)?;
        }
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Write coords from the geometry, fitting Béziers when requested.
    fn write_geometry_coords<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        geometry: &LineString,
    ) -> Result<()> {
        let coords = if let Some(bezier_error) = self.bezier_write_error {
            let bezier = BezierString::from_line_string(geometry.clone(), bezier_error.get())?;
            let final_vertex_flags = if geometry.is_closed() {
                COORD_FLAGS_RING_END
            } else {
                0
            };
            file_coords_from_bezier(&bezier, final_vertex_flags)?
        } else {
            geometry
                .coords()
                .map(|coord| Ok((to_file_coords(*coord)?, 0)))
                .collect::<Result<Vec<_>>>()?
        };
        super::write_raw_coords(writer, &coords)
    }

    /// Parse a line object. The reader should be positioned right after the `<object>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: WeakLinePathSymbol,
    ) -> Result<Self> {
        let mut tags = HashMap::new();
        let mut raw_map_coords = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"coords" => {
                        let num_coords = try_get_attr_raw(&bytes_start, "count")
                            .ok()
                            .flatten()
                            .unwrap_or(0);
                        raw_map_coords.reserve(num_coords);
                    }
                    b"tags" => tags = super::parse_tags(reader)?,
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
                    return Err(Error::UnexpectedEof(OmapSection::LineObject));
                }
                _ => (),
            }
        }
        Ok(Self {
            tags,
            symbol,
            bezier_write_error: None,
            geometry: OnceCell::new(),
            bezier: OnceCell::new(),
            raw_map_coords,
            is_coords_touched: false,
        })
    }

    fn flattened_geometry(&self, allowed_error: f64) -> Result<LineString> {
        if self.raw_map_coords.len() < 2 {
            return Err(Error::ObjectError);
        }

        // Flatten the cached path when there already is one — same result, one
        // reconstruction less. Not populated here: a consumer that only ever
        // asks for the flattened geometry should not pay to keep the path.
        if let Some(bezier) = self.bezier.get() {
            return Ok(bezier.geometry().to_line_string(allowed_error)?);
        }

        Ok(bezier_from_raw_coords(&self.raw_map_coords)
            .ok_or(Error::ObjectError)?
            .geometry()
            .to_line_string(allowed_error)?)
    }
}

pub(crate) fn reverse_raw_line_coords(coords: &[FileCoord]) -> Vec<FileCoord> {
    // iterate through and check the flags
    // Curve-start flags must move to the other end of each Bézier. Ring-end
    // flags can only exist at the end and must move to the new end. All other
    // flags, including dash-point flags, stay attached to their coordinates.
    let mut new_xml = Vec::with_capacity(coords.len());

    let mut end_flag = 0;
    for i in (0..coords.len()).rev() {
        let (coord, mut flag) = coords[i];
        // remove a possible bezier flag
        flag -= flag & COORD_FLAG_CURVE_START;

        if i == coords.len() - 1 {
            end_flag += flag & COORD_FLAGS_RING_END;
            flag -= end_flag;
        }
        if i > 2 {
            // Move a curve-start flag from the opposite end of the cubic.
            let (_, bez_flag) = coords[i - 3];
            flag |= bez_flag & COORD_FLAG_CURVE_START;
        } else if i == 0 {
            flag |= end_flag;
        }
        new_xml.push((coord, flag));
    }
    new_xml
}

#[cfg(test)]
mod tests {
    use geo_types::{Coord, LineString};
    use linestring2bezier::BezierSegment;
    use quick_xml::{Reader, Writer, events::Event};

    use super::LineObject;
    use crate::{
        Error, Result,
        geo_referencing::{AffineMapTransform, GeoRef, MapTransform},
        symbols::WeakLinePathSymbol,
    };

    /// A curve from (0, 0) via (0, -1) and (1, -1) to (1, 0), both ends dashed.
    const CURVE_XML: &str =
        r#"<object><coords count="4">0 0 33;0 1000;1000 1000;1000 0 32;</coords></object>"#;

    fn parse_line(xml: &str) -> Result<LineObject> {
        let mut reader = Reader::from_str(xml);
        let event = reader.read_event()?;
        assert!(matches!(event, Event::Start(_)), "expected an object start");

        LineObject::parse(&mut reader, WeakLinePathSymbol::Line(std::rc::Weak::new()))
    }

    /// An affine transform that only translates, built the only way the public
    /// API allows: as the difference between two map reference points.
    fn translation(dx: f64, dy: f64) -> Result<AffineMapTransform> {
        let before = GeoRef::new(10_000).get_transform();
        let mut after = GeoRef::new(10_000);
        after.map_ref_point = Coord { x: dx, y: dy };
        MapTransform::affine_between(&before, &after.get_transform())
    }

    fn first_segment_start(line: &LineObject) -> Result<Coord> {
        line.bezier_geometry()
            .ok_or(Error::ObjectError)?
            .geometry()
            .segments()
            .next()
            .map(BezierSegment::start)
            .ok_or(Error::ObjectError)
    }

    #[test]
    fn bezier_geometry_is_cached_until_the_coords_change() -> Result<()> {
        let mut line = parse_line(CURVE_XML)?;

        let first = std::ptr::from_ref(line.bezier_geometry().ok_or(Error::ObjectError)?);
        let second = std::ptr::from_ref(line.bezier_geometry().ok_or(Error::ObjectError)?);
        assert!(
            std::ptr::eq(first, second),
            "a second call must hand out the cached path, not a rebuilt one"
        );

        // Both of these move the raw coords without marking them as touched.
        line.apply_affine(&translation(10., 20.)?);
        assert_eq!(
            first_segment_start(&line)?,
            Coord { x: 10., y: 20. },
            "the cache must not survive apply_affine"
        );

        line.reverse_linestring();
        assert_eq!(
            first_segment_start(&line)?,
            Coord { x: 11., y: 20. },
            "the cache must not survive reverse_linestring"
        );

        // Touching the coords drops the path for good.
        let _ = line.get_geometry_mut(0.1)?;
        assert!(line.bezier_geometry().is_none());
        Ok(())
    }

    #[test]
    fn bezier_geometry_or_straight_keeps_the_exact_path() -> Result<()> {
        let line = parse_line(CURVE_XML)?;

        let exact = std::ptr::from_ref(line.bezier_geometry().ok_or(Error::ObjectError)?);
        let always = std::ptr::from_ref(line.bezier_geometry_or_straight()?);
        assert!(
            std::ptr::eq(exact, always),
            "must not fall back while the raw coords are intact"
        );
        assert!(
            line.bezier_geometry_or_straight()?
                .geometry()
                .segments()
                .any(BezierSegment::is_bezier_curve),
            "the curve must survive"
        );
        Ok(())
    }

    #[test]
    fn bezier_geometry_or_straight_falls_back_to_straight_segments() -> Result<()> {
        let vertices = vec![
            Coord { x: 0., y: 0. },
            Coord { x: 1., y: 0. },
            Coord { x: 1., y: 1. },
        ];
        let line = LineObject::new(
            WeakLinePathSymbol::Line(std::rc::Weak::new()),
            LineString::new(vertices.clone()),
        );

        assert!(
            line.bezier_geometry().is_none(),
            "an object built in memory has no raw coords"
        );

        let path = line.bezier_geometry_or_straight()?;
        assert_eq!(path.geometry().num_segments(), 2);
        assert!(
            path.geometry()
                .segments()
                .all(|segment| matches!(segment, BezierSegment::Line(_))),
            "no handles can be invented"
        );
        assert_eq!(path.vertex_is_dash_point(), [false, false, false]);
        assert_eq!(path.geometry().to_line_string(0.01)?.0, vertices);
        Ok(())
    }

    #[test]
    fn bezier_geometry_or_straight_follows_the_edited_geometry() -> Result<()> {
        let mut line = parse_line(CURVE_XML)?;
        // Cache the exact path first, so handing out a stale one would show.
        assert!(line.bezier_geometry().is_some());

        line.get_geometry_mut(0.01)?.0 = vec![Coord { x: 0., y: 0. }, Coord { x: 5., y: 5. }];

        assert!(line.bezier_geometry().is_none());
        let path = line.bezier_geometry_or_straight()?;
        assert_eq!(path.geometry().num_segments(), 1);
        assert_eq!(path.vertex_is_dash_point(), [false, false]);
        assert_eq!(
            path.geometry().segments().next().map(BezierSegment::end),
            Some(Coord { x: 5., y: 5. })
        );
        Ok(())
    }

    #[test]
    fn flattening_is_unaffected_by_a_cached_path() -> Result<()> {
        let cached = parse_line(CURVE_XML)?;
        assert!(cached.bezier_geometry().is_some(), "expected a cached path");
        let plain = parse_line(CURVE_XML)?;

        assert_eq!(cached.get_geometry(0.01)?, plain.get_geometry(0.01)?);
        Ok(())
    }

    #[test]
    fn parsed_line_geometry_is_initialized_lazily() -> Result<()> {
        let mut reader = Reader::from_str(
            r#"<object><coords count="4">0 0 33;0 1000;1000 1000;1000 0 32;</coords></object>"#,
        );
        let event = reader.read_event()?;
        assert!(matches!(event, Event::Start(_)));

        let mut line =
            LineObject::parse(&mut reader, WeakLinePathSymbol::Line(std::rc::Weak::new()))?;
        assert!(line.geometry.get().is_none());
        assert!(!line.is_coords_touched);
        assert!(line.bezier_geometry().is_some());

        let mut writer = Writer::new(Vec::new());
        line.write_content(&mut writer, None)?;
        assert!(line.geometry.get().is_none());
        let output = String::from_utf8(writer.into_inner())?;
        assert!(output.contains("0 0 33;0 1000;1000 1000;1000 0 32;"));

        assert!(line.get_geometry(0.0).is_err());
        assert!(line.geometry.get().is_none());
        assert!(!line.is_coords_touched);

        assert!(!line.get_geometry(0.1)?.0.is_empty());
        assert!(line.geometry.get().is_some());
        assert!(!line.is_coords_touched);

        let _ = line.get_geometry_mut(0.2)?;
        assert!(line.is_coords_touched);
        assert!(line.bezier_geometry().is_none());
        Ok(())
    }

    #[test]
    fn parsed_empty_line_has_no_geometry_and_is_not_written() -> Result<()> {
        for xml in [
            r#"<object><coords count="0"></coords></object>"#,
            r#"<object><coords count="1">0 0;</coords></object>"#,
        ] {
            let mut reader = Reader::from_str(xml);
            let event = reader.read_event()?;
            assert!(matches!(event, Event::Start(_)));

            let line =
                LineObject::parse(&mut reader, WeakLinePathSymbol::Line(std::rc::Weak::new()))?;
            assert!(line.geometry.get().is_none());
            assert!(line.geometry_is_empty());
            assert!(line.bezier_geometry().is_none());
            assert!(line.bezier_geometry_or_straight().is_err());
            assert!(line.get_geometry(0.1).is_err());

            let mut writer = Writer::new(Vec::new());
            line.write_content(&mut writer, None)?;
            assert!(line.geometry.get().is_none());
            assert!(writer.into_inner().is_empty());
        }
        Ok(())
    }
}
