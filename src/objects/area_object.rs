use std::collections::HashMap;

use geo_types::{Coord, Polygon};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{
    BezierPath, COORD_FLAGS_RING_END, FileCoord, FlattenedPath, bezier_from_file_coords,
    file_coords_from_bezier,
};
use crate::{
    Error, NonNegativeF64, OmapSection, Result,
    symbols::{Symbol, SymbolSet, WeakAreaPathSymbol},
    utils::{from_file_coords, to_file_coords, try_get_attr_raw, try_transform_position},
};

/// A polygon whose exterior and interior rings retain straight and cubic
/// Bézier segments and dash-point metadata.
#[derive(Debug, Clone)]
pub struct BezierPolygon {
    /// The polygon's exterior ring.
    exterior: BezierPath,
    /// The polygon's interior rings.
    interiors: Vec<BezierPath>,
}

impl BezierPolygon {
    /// Construct a polygon, closing every nonempty ring with a straight segment.
    ///
    /// # Errors
    ///
    /// Returns an error when a path is invalid.
    pub fn new(mut exterior: BezierPath, mut interiors: Vec<BezierPath>) -> Result<Self> {
        exterior.validate()?;
        exterior.close();
        interiors.retain(|ring| !ring.is_empty());
        for ring in &mut interiors {
            ring.validate()?;
            ring.close();
        }
        Ok(Self {
            exterior,
            interiors,
        })
    }

    /// Construct an empty polygon.
    pub fn empty() -> Self {
        Self {
            exterior: BezierPath::empty(),
            interiors: Vec::new(),
        }
    }

    /// Fit smooth Bézier paths to every ring within `allowed_error`.
    ///
    /// Every fitted vertex initially has its forced dash-point state disabled.
    /// Empty interior rings are omitted, and an empty exterior produces an
    /// empty polygon.
    ///
    /// Unlike [`From<Polygon>`], which preserves every input segment as a
    /// straight line, this method may replace several input segments with one
    /// cubic Bézier segment.
    ///
    /// # Errors
    ///
    /// Returns an error when a nonempty ring contains fewer than two
    /// coordinates or `allowed_error` is too small for fitting.
    pub fn fit_polygon(polygon: Polygon, allowed_error: NonNegativeF64) -> Result<Self> {
        let (exterior, interiors) = polygon.into_inner();
        let exterior = if exterior.0.is_empty() {
            BezierPath::empty()
        } else {
            BezierPath::fit_line_string(exterior, allowed_error)?
        };
        let interiors = interiors
            .into_iter()
            .filter(|ring| !ring.0.is_empty())
            .map(|ring| BezierPath::fit_line_string(ring, allowed_error))
            .collect::<Result<Vec<_>>>()?;
        Self::new(exterior, interiors)
    }

    /// Get the exterior ring.
    pub fn exterior(&self) -> &BezierPath {
        &self.exterior
    }

    /// Get the interior rings.
    pub fn interiors(&self) -> &[BezierPath] {
        &self.interiors
    }

    /// Get mutable access to the exterior ring.
    pub fn exterior_mut(&mut self) -> &mut BezierPath {
        &mut self.exterior
    }

    /// Get mutable access to the interior rings.
    pub fn interiors_mut(&mut self) -> &mut [BezierPath] {
        &mut self.interiors
    }

    /// Consume the polygon and return its rings.
    pub fn into_parts(self) -> (BezierPath, Vec<BezierPath>) {
        (self.exterior, self.interiors)
    }

    /// Return whether the exterior ring is empty.
    pub fn is_empty(&self) -> bool {
        self.exterior.is_empty()
    }

    /// Flatten every ring while retaining dash-point metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when a tolerance is too small or a ring is invalid.
    pub fn flatten(&self, allowed_error: NonNegativeF64) -> Result<FlattenedPolygon> {
        self.validate()?;
        let exterior = self.exterior.flatten(allowed_error)?;
        let interiors = self
            .interiors
            .iter()
            .filter(|ring| !ring.is_empty())
            .map(|ring| ring.flatten(allowed_error))
            .collect::<Result<Vec<_>>>()?;
        FlattenedPolygon::new(exterior, interiors)
    }

    /// Transform every ring coordinate while preserving topology and metadata.
    pub fn transform<F>(mut self, transform: F) -> Self
    where
        F: Fn(Coord) -> Coord,
    {
        self.exterior = self.exterior.transform(&transform);
        self.interiors = self
            .interiors
            .into_iter()
            .map(|ring| ring.transform(&transform))
            .collect();
        self
    }

    /// Try to transform every ring coordinate, stopping at the first error.
    ///
    /// # Errors
    ///
    /// Returns the first error produced by `transform`.
    pub fn try_transform<E, F>(mut self, transform: F) -> std::result::Result<Self, E>
    where
        F: Fn(Coord) -> std::result::Result<Coord, E>,
    {
        self.exterior = self.exterior.try_transform(&transform)?;
        self.interiors = self
            .interiors
            .into_iter()
            .map(|ring| ring.try_transform(&transform))
            .collect::<std::result::Result<_, _>>()?;
        Ok(self)
    }

    /// Reverse the winding order of every ring.
    pub fn reverse(&mut self) {
        self.exterior.reverse();
        for ring in &mut self.interiors {
            ring.reverse();
        }
    }

    /// Validate ring topology and path metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when a path is invalid or a nonempty ring is open.
    pub fn validate(&self) -> Result<()> {
        self.exterior.validate()?;
        if !self.exterior.is_empty() && !self.exterior.is_closed() {
            return Err(Error::OpenPolygonRing);
        }
        for ring in &self.interiors {
            ring.validate()?;
            if !ring.is_empty() && !ring.is_closed() {
                return Err(Error::OpenPolygonRing);
            }
        }
        Ok(())
    }
}

impl From<Polygon> for BezierPolygon {
    fn from(polygon: Polygon) -> Self {
        let (exterior, interiors) = polygon.into_inner();
        Self {
            exterior: exterior.into(),
            interiors: interiors.into_iter().map(BezierPath::from).collect(),
        }
    }
}

/// An owned flattened polygon whose rings retain dash-point metadata.
#[derive(Debug, Clone)]
pub struct FlattenedPolygon {
    /// The flattened exterior ring.
    exterior: FlattenedPath,
    /// The flattened interior rings.
    interiors: Vec<FlattenedPath>,
}

impl FlattenedPolygon {
    /// Construct a flattened polygon from closed rings.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OpenPolygonRing`] when a nonempty ring is not closed.
    pub fn new(exterior: FlattenedPath, interiors: Vec<FlattenedPath>) -> Result<Self> {
        if (!exterior.geometry().0.is_empty() && !exterior.geometry().is_closed())
            || interiors
                .iter()
                .any(|ring| !ring.geometry().0.is_empty() && !ring.geometry().is_closed())
        {
            return Err(Error::OpenPolygonRing);
        }
        Ok(Self {
            exterior,
            interiors,
        })
    }

    /// Get the exterior ring.
    pub fn exterior(&self) -> &FlattenedPath {
        &self.exterior
    }

    /// Get the interior rings.
    pub fn interiors(&self) -> &[FlattenedPath] {
        &self.interiors
    }

    /// Consume the polygon and return its rings.
    pub fn into_parts(self) -> (FlattenedPath, Vec<FlattenedPath>) {
        (self.exterior, self.interiors)
    }

    /// Consume the flattened polygon and discard dash metadata.
    pub fn into_polygon(self) -> Polygon {
        let exterior = self.exterior.into_parts().0;
        let interiors = self
            .interiors
            .into_iter()
            .map(|ring| ring.into_parts().0)
            .collect();
        Polygon::new(exterior, interiors)
    }
}

impl From<FlattenedPolygon> for BezierPolygon {
    fn from(polygon: FlattenedPolygon) -> Self {
        let (exterior, interiors) = polygon.into_parts();
        Self {
            exterior: exterior.into(),
            interiors: interiors.into_iter().map(BezierPath::from).collect(),
        }
    }
}

/// A fill pattern rotation and origin used by area objects.
#[derive(Debug, Clone, Default)]
pub struct PatternRotation {
    /// Rotation of the fill pattern in radians.
    pub rotation: f64,
    /// Origin coordinate for the pattern.
    pub coord: Coord,
}

/// An area object whose geometry retains straight and cubic rings.
#[derive(Debug, Clone)]
pub struct AreaObject {
    /// The tags associated with the object.
    pub tags: HashMap<String, String>,
    /// The fill-pattern rotation and origin.
    pub pattern_rotation: PatternRotation,
    /// The area or combined-area symbol used to render this object.
    pub symbol: WeakAreaPathSymbol,
    geometry: BezierPolygon,
}

impl AreaObject {
    /// Create an area object from a Bézier or `geo_types` polygon.
    pub fn new(symbol: impl Into<WeakAreaPathSymbol>, geometry: impl Into<BezierPolygon>) -> Self {
        Self {
            tags: HashMap::new(),
            pattern_rotation: PatternRotation::default(),
            symbol: symbol.into(),
            geometry: geometry.into(),
        }
    }

    /// Get the mixed straight/cubic polygon.
    pub fn geometry(&self) -> &BezierPolygon {
        &self.geometry
    }

    /// Mutably access the mixed straight/cubic polygon.
    pub fn geometry_mut(&mut self) -> &mut BezierPolygon {
        &mut self.geometry
    }

    /// Consume the object and return its geometry.
    pub fn into_geometry(self) -> BezierPolygon {
        self.geometry
    }

    /// Create an owned flattened polygon with dash-point metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when flattening fails.
    pub fn flatten(&self, allowed_error: NonNegativeF64) -> Result<FlattenedPolygon> {
        self.geometry.flatten(allowed_error)
    }

    /// Replace the rings with straight flattened segments.
    pub fn replace_with_flattened(&mut self, geometry: FlattenedPolygon) {
        self.geometry = geometry.into();
    }

    /// Permanently replace curves with their flattened straight segments.
    ///
    /// # Errors
    ///
    /// Returns an error when flattening fails.
    pub fn flatten_in_place(&mut self, allowed_error: NonNegativeF64) -> Result<()> {
        self.geometry = self.flatten(allowed_error)?.into();
        Ok(())
    }

    /// Create an area object for use as a point-symbol element.
    pub fn new_element(geometry: impl Into<BezierPolygon>) -> Self {
        Self::new(WeakAreaPathSymbol::Area(std::rc::Weak::new()), geometry)
    }

    pub(crate) fn geometry_is_empty(&self) -> bool {
        self.geometry.is_empty()
    }

    /// Transform the polygon and pattern orientation.
    pub fn transform<F>(&mut self, transform: F)
    where
        F: Fn(Coord) -> Coord,
    {
        let geometry =
            std::mem::replace(&mut self.geometry, BezierPolygon::empty()).transform(&transform);
        let (pattern_coord, pattern_rotation, _) =
            crate::utils::transform_position(self.pattern_rotation.coord, &transform);

        self.geometry = geometry;
        self.pattern_rotation.coord = pattern_coord;
        self.pattern_rotation.rotation += pattern_rotation;
    }

    /// Try to transform the polygon and pattern orientation.
    ///
    /// # Errors
    ///
    /// Returns the first transformation error. The object is unchanged on
    /// failure.
    pub fn try_transform<E, F>(&mut self, transform: F) -> std::result::Result<(), E>
    where
        F: Fn(Coord) -> std::result::Result<Coord, E>,
    {
        let geometry = self.geometry.clone().try_transform(&transform)?;
        let (pattern_coord, pattern_rotation, _) =
            try_transform_position(self.pattern_rotation.coord, &transform)?;

        self.geometry = geometry;
        self.pattern_rotation.coord = pattern_coord;
        self.pattern_rotation.rotation += pattern_rotation;
        Ok(())
    }

    /// Reverse the winding order of every ring.
    pub fn reverse(&mut self) {
        self.geometry.reverse();
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let index = match &self.symbol {
            WeakAreaPathSymbol::Area(weak) => weak.upgrade().and_then(|symbol| {
                symbol_set.iter().position(|candidate| match candidate {
                    Symbol::Area(reference) => reference.as_ptr() == symbol.as_ptr(),
                    _ => false,
                })
            }),
            WeakAreaPathSymbol::CombinedArea(weak) => weak.upgrade().and_then(|symbol| {
                symbol_set.iter().position(|candidate| match candidate {
                    Symbol::CombinedArea(reference) => reference.as_ptr() == symbol.as_ptr(),
                    _ => false,
                })
            }),
        }
        .map_or(-1, |index| index as i32);

        self.write_content(writer, Some(index))
    }

    /// Write a full object element for use inside a point symbol.
    pub(crate) fn write_as_element<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        self.write_content(writer, None)
    }

    fn write_content<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_index: Option<i32>,
    ) -> Result<()> {
        if self.geometry_is_empty() {
            return Ok(());
        }
        self.geometry.validate()?;

        let mut start = BytesStart::new("object").with_attributes([("type", "1")]);
        if let Some(symbol_index) = symbol_index {
            start.push_attribute(("symbol", symbol_index.to_string().as_str()));
        }
        writer.write_event(Event::Start(start))?;

        if !self.tags.is_empty() && symbol_index.is_some() {
            super::write_tags(writer, &self.tags)?;
        }

        let mut all_coords =
            file_coords_from_bezier(&self.geometry.exterior, COORD_FLAGS_RING_END)?;
        for ring in self
            .geometry
            .interiors
            .iter()
            .filter(|ring| !ring.is_empty())
        {
            all_coords.extend(file_coords_from_bezier(ring, COORD_FLAGS_RING_END)?);
        }
        super::write_file_coords(writer, &all_coords)?;
        self.write_pattern(writer)?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    fn write_pattern<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let pattern = &self.pattern_rotation;
        let mut start = BytesStart::new("pattern");
        start.push_attribute(("rotation", pattern.rotation.to_string().as_str()));
        writer.write_event(Event::Start(start))?;
        let coord = to_file_coords(pattern.coord)?;
        writer.write_event(Event::Empty(BytesStart::new("coord").with_attributes([
            ("x", coord.x.to_string().as_str()),
            ("y", coord.y.to_string().as_str()),
        ])))?;
        writer.write_event(Event::End(BytesEnd::new("pattern")))?;
        Ok(())
    }

    /// Parse an area object through its closing `object` element.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: WeakAreaPathSymbol,
    ) -> Result<Self> {
        let mut tags = HashMap::new();
        let mut pattern_rotation = PatternRotation::default();
        let mut file_coords = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(start) => match start.local_name().as_ref() {
                    b"coords" => {
                        let count = try_get_attr_raw(&start, "count")
                            .ok()
                            .flatten()
                            .unwrap_or(0);
                        file_coords.reserve(count);
                    }
                    b"pattern" => {
                        pattern_rotation.rotation = try_get_attr_raw(&start, "rotation")
                            .ok()
                            .flatten()
                            .unwrap_or(pattern_rotation.rotation);
                    }
                    b"tags" => tags = super::parse_tags(reader)?,
                    b"coord" => {
                        let x = try_get_attr_raw(&start, "x")?.unwrap_or(0);
                        let y = try_get_attr_raw(&start, "y")?.unwrap_or(0);
                        pattern_rotation.coord = from_file_coords(Coord { x, y });
                    }
                    _ => (),
                },
                Event::End(end) if end.local_name().as_ref() == b"object" => break,
                Event::Text(text) => super::parse_file_coords(text.as_ref(), &mut file_coords)?,
                Event::Eof => return Err(Error::UnexpectedEof(OmapSection::AreaObject)),
                _ => (),
            }
        }

        Ok(Self {
            tags,
            pattern_rotation,
            symbol,
            geometry: bezier_polygon_from_file_coords(&file_coords)
                .unwrap_or_else(BezierPolygon::empty),
        })
    }
}

fn bezier_polygon_from_file_coords(coords: &[FileCoord]) -> Option<BezierPolygon> {
    let mut rings = Vec::new();
    let mut ring_start = 0;

    for (index, (_, flags)) in coords.iter().enumerate() {
        if flags & COORD_FLAGS_RING_END != 0 {
            if let Some(ring) = bezier_from_file_coords(&coords[ring_start..=index]) {
                rings.push(ring);
            }
            ring_start = index + 1;
        }
    }
    if ring_start < coords.len()
        && let Some(ring) = bezier_from_file_coords(&coords[ring_start..])
    {
        rings.push(ring);
    }

    let mut rings = rings.into_iter();
    BezierPolygon::new(rings.next()?, rings.collect()).ok()
}

#[cfg(test)]
mod tests {
    use geo_types::{LineString, Polygon};
    use quick_xml::{Reader, Writer, events::Event};

    use super::{AreaObject, BezierPolygon};
    use crate::{NonNegativeF64, Result, objects::BezierSegment, symbols::WeakAreaPathSymbol};

    #[test]
    fn fitted_polygon_can_construct_area_object() -> Result<()> {
        let polygon = Polygon::new(
            LineString::from(vec![
                (0.0, 2.0),
                (1.4, 1.4),
                (2.0, 0.0),
                (1.4, -1.4),
                (0.0, -2.0),
                (-1.4, -1.4),
                (-2.0, 0.0),
                (-1.4, 1.4),
                (0.0, 2.0),
            ]),
            vec![LineString::from(vec![
                (0.0, 1.0),
                (0.7, 0.7),
                (1.0, 0.0),
                (0.7, -0.7),
                (0.0, -1.0),
                (-0.7, -0.7),
                (-1.0, 0.0),
                (-0.7, 0.7),
                (0.0, 1.0),
            ])],
        );
        let polygon = BezierPolygon::fit_polygon(polygon, NonNegativeF64::clamped_from(0.2))?;
        let area = AreaObject::new(WeakAreaPathSymbol::Area(std::rc::Weak::new()), polygon);

        assert_eq!(area.geometry().interiors().len(), 1);
        for ring in std::iter::once(area.geometry().exterior()).chain(area.geometry().interiors()) {
            assert!(ring.is_closed());
            assert_eq!(ring.num_vertices(), ring.num_segments() + 1);
            assert!(ring.vertex_is_dash_point().iter().all(|flag| !flag));
        }
        assert!(
            area.geometry()
                .exterior()
                .geometry()
                .segments()
                .any(BezierSegment::is_bezier_curve)
        );
        Ok(())
    }

    #[test]
    fn parsed_polygon_owns_closed_bezier_rings_and_dash_points() -> Result<()> {
        let mut reader = Reader::from_str(
            r#"<object><coords count="7">0 0 32;2000 0;1000 1000 2;500 250 32;1000 250;750 750;500 250 50;</coords><pattern rotation="0"></pattern></object>"#,
        );
        assert!(matches!(reader.read_event()?, Event::Start(_)));

        let area = AreaObject::parse(&mut reader, WeakAreaPathSymbol::Area(std::rc::Weak::new()))?;
        assert!(area.geometry().exterior().is_closed());
        assert_eq!(area.geometry().interiors().len(), 1);
        assert!(area.geometry().interiors()[0].is_closed());
        for ring in std::iter::once(area.geometry().exterior()).chain(area.geometry().interiors()) {
            assert_eq!(ring.num_vertices(), ring.num_segments() + 1);
        }
        assert_eq!(
            area.geometry().exterior().vertex_is_dash_point(),
            [true, false, false, true]
        );

        let flattened = area.flatten(NonNegativeF64::clamped_from(0.1))?;
        assert_eq!(flattened.interiors().len(), 1);
        for ring in std::iter::once(flattened.exterior()).chain(flattened.interiors()) {
            assert_eq!(ring.num_vertices(), ring.num_segments() + 1);
        }
        assert_eq!(
            flattened.exterior().vertex_is_dash_point().len(),
            flattened.exterior().geometry().0.len()
        );

        let mut writer = Writer::new(Vec::new());
        area.write_content(&mut writer, None)?;
        let output = String::from_utf8(writer.into_inner())?;
        assert!(output.contains("0 0 32;2000 0;1000 1000;0 0 50;"));
        assert!(output.contains("500 250 32;1000 250;750 750;500 250 50;"));
        Ok(())
    }

    #[test]
    fn flatten_in_place_replaces_curves_with_lines() -> Result<()> {
        let mut reader = Reader::from_str(
            r#"<object><coords count="5">0 0 1;0 1000;1000 1000;1000 0;0 0 2;</coords><pattern rotation="0"></pattern></object>"#,
        );
        assert!(matches!(reader.read_event()?, Event::Start(_)));
        let mut area =
            AreaObject::parse(&mut reader, WeakAreaPathSymbol::Area(std::rc::Weak::new()))?;

        area.flatten_in_place(NonNegativeF64::clamped_from(0.1))?;
        assert!(
            area.geometry()
                .exterior()
                .geometry()
                .segments()
                .all(|segment| matches!(segment, BezierSegment::Line(_)))
        );
        Ok(())
    }
}
