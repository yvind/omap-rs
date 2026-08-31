use std::collections::HashMap;

use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{
    BezierPath, COORD_FLAGS_RING_END, FlattenedPath, bezier_from_file_coords,
    file_coords_from_bezier,
};
use crate::{
    Error, NonNegativeF64, OmapSection, Result,
    symbols::{LinePathSymbolId, SymbolSet},
    utils::try_get_attr_raw,
};

/// A line object whose geometry retains straight and cubic segments.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineObject {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    #[expect(
        clippy::box_collection,
        reason = "the map header is 48 bytes inline and most objects carry no tags"
    )]
    tags: Option<Box<HashMap<String, String>>>,
    /// The line or combined-line symbol used to render this object.
    pub symbol: Option<LinePathSymbolId>,
    geometry: BezierPath,
}

impl LineObject {
    /// Create a line object from a Bézier path, flattened path, or line string.
    pub fn new(symbol: Option<LinePathSymbolId>, geometry: impl Into<BezierPath>) -> Self {
        Self {
            tags: None,
            symbol,
            geometry: geometry.into(),
        }
    }

    /// The tags associated with the object.
    pub fn tags(&self) -> &HashMap<String, String> {
        match &self.tags {
            Some(tags) => tags,
            None => super::empty_tags(),
        }
    }

    /// Mutably access the tags, allocating the map on first use.
    pub fn tags_mut(&mut self) -> &mut HashMap<String, String> {
        self.tags.get_or_insert_with(Box::default)
    }

    /// Get the mixed straight/cubic geometry.
    pub fn geometry(&self) -> &BezierPath {
        &self.geometry
    }

    /// Mutably access the mixed straight/cubic geometry.
    pub fn geometry_mut(&mut self) -> &mut BezierPath {
        &mut self.geometry
    }

    /// Consume the object and return its geometry.
    pub fn into_geometry(self) -> BezierPath {
        self.geometry
    }

    /// Create an owned flattened path with dash-point metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when the tolerance is too small or the path is invalid.
    pub fn flatten(&self, allowed_error: NonNegativeF64) -> Result<FlattenedPath> {
        self.geometry.flatten(allowed_error)
    }

    /// Replace the path with straight segments from flattened geometry.
    pub fn replace_with_flattened(&mut self, geometry: FlattenedPath) {
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

    /// Create a line object for use as a point-symbol element.
    pub fn new_element(geometry: impl Into<BezierPath>) -> Self {
        Self::new(None, geometry)
    }

    pub(crate) fn geometry_is_empty(&self) -> bool {
        self.geometry.is_empty()
    }

    /// Transform the geometry while preserving curves and dash points.
    pub fn transform<F>(&mut self, transform: F)
    where
        F: Fn(Coord) -> Coord,
    {
        self.geometry =
            std::mem::replace(&mut self.geometry, BezierPath::empty()).transform(transform);
    }

    /// Try to transform the geometry while preserving curves and dash points.
    ///
    /// # Errors
    ///
    /// Returns the first error produced by `transform`. The object is unchanged
    /// when transformation fails.
    pub fn try_transform<E, F>(&mut self, transform: F) -> std::result::Result<(), E>
    where
        F: Fn(Coord) -> std::result::Result<Coord, E>,
    {
        self.geometry = self.geometry.clone().try_transform(transform)?;
        Ok(())
    }

    /// Reverse the path while keeping dash flags attached to their vertices.
    pub fn reverse(&mut self) {
        self.geometry.reverse();
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let index = symbol_set.file_index(self.symbol);

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

        let mut start = BytesStart::new("object").with_attributes([("type", "1")]);
        if let Some(symbol_index) = symbol_index {
            start.push_attribute(("symbol", symbol_index.to_string().as_str()));
        }
        writer.write_event(Event::Start(start))?;

        if !self.tags().is_empty() && symbol_index.is_some() {
            super::write_tags(writer, self.tags())?;
        }

        let final_flags = if self.geometry.is_closed() {
            COORD_FLAGS_RING_END
        } else {
            0
        };
        let coords = file_coords_from_bezier(&self.geometry, final_flags)?;
        super::write_file_coords(writer, &coords)?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Parse a line object through its closing `object` element.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: Option<LinePathSymbolId>,
    ) -> Result<Self> {
        let mut tags = None;
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
                    b"tags" => tags = super::parse_tags(reader)?,
                    _ => (),
                },
                Event::End(end) if end.local_name().as_ref() == b"object" => break,
                Event::Text(text) => super::parse_file_coords(text.as_ref(), &mut file_coords)?,
                Event::Eof => return Err(Error::UnexpectedEof(OmapSection::LineObject)),
                _ => (),
            }
        }

        Ok(Self {
            tags,
            symbol,
            geometry: bezier_from_file_coords(&file_coords).unwrap_or_else(BezierPath::empty),
        })
    }
}

#[cfg(test)]
mod tests {
    use geo_types::LineString;
    use quick_xml::{Reader, Writer, events::Event};

    use super::LineObject;
    use crate::{
        NonNegativeF64, Result,
        objects::{BezierPath, BezierSegment, FlattenedPath},
    };

    #[test]
    fn fitted_line_string_can_construct_line_object() -> Result<()> {
        let path = BezierPath::fit_line_string(
            LineString::from(vec![
                (0.0, 0.0),
                (1.0, 1.0),
                (2.0, 0.0),
                (3.0, -1.0),
                (4.0, 0.0),
            ]),
            NonNegativeF64::clamped_from(0.1),
        )?;
        let line = LineObject::new(None, path);

        assert!(
            line.geometry()
                .geometry()
                .segments()
                .any(BezierSegment::is_bezier_curve)
        );
        assert_eq!(
            line.geometry().num_vertices(),
            line.geometry().num_segments() + 1
        );
        Ok(())
    }

    #[test]
    fn parsed_line_owns_exact_beziers_and_dash_points() -> Result<()> {
        let mut reader = Reader::from_str(
            r#"<object><coords count="4">0 0 33;0 1000;1000 1000;1000 0 32;</coords></object>"#,
        );
        assert!(matches!(reader.read_event()?, Event::Start(_)));

        let line = LineObject::parse(&mut reader, None)?;
        assert!(matches!(
            line.geometry().geometry().segments().next(),
            Some(BezierSegment::Bezier(_))
        ));
        assert_eq!(line.geometry().vertex_is_dash_point(), [true, true]);
        assert_eq!(
            line.geometry().num_vertices(),
            line.geometry().num_segments() + 1
        );

        let flattened = line.flatten(NonNegativeF64::clamped_from(0.1))?;
        assert_eq!(
            flattened.vertex_is_dash_point().len(),
            flattened.geometry().0.len()
        );
        assert_eq!(flattened.num_vertices(), flattened.num_segments() + 1);
        assert_eq!(flattened.vertex_is_dash_point().first(), Some(&true));
        assert_eq!(flattened.vertex_is_dash_point().last(), Some(&true));

        let mut writer = Writer::new(Vec::new());
        line.write_content(&mut writer, None)?;
        let output = String::from_utf8(writer.into_inner())?;
        assert!(output.contains("0 0 33;0 1000;1000 1000;1000 0 32;"));
        Ok(())
    }

    #[test]
    fn replacing_with_flattened_geometry_makes_straight_segments() -> Result<()> {
        let flattened = FlattenedPath::new(
            vec![(0., 0.), (1., 0.), (2., 0.)].into(),
            vec![true, false, true],
        )?;
        let mut line = LineObject::new(None, flattened.clone());
        assert!(
            line.geometry()
                .geometry()
                .segments()
                .all(|segment| matches!(segment, BezierSegment::Line(_)))
        );

        line.replace_with_flattened(flattened);
        assert_eq!(line.geometry().vertex_is_dash_point(), [true, false, true]);
        assert_eq!(
            line.geometry().num_vertices(),
            line.geometry().num_segments() + 1
        );
        Ok(())
    }

    #[test]
    fn parsed_empty_line_is_not_written() -> Result<()> {
        let mut reader = Reader::from_str(r#"<object><coords count="1">0 0;</coords></object>"#);
        assert!(matches!(reader.read_event()?, Event::Start(_)));
        let line = LineObject::parse(&mut reader, None)?;

        assert!(line.geometry().is_empty());
        let mut writer = Writer::new(Vec::new());
        line.write_content(&mut writer, None)?;
        assert!(writer.into_inner().is_empty());
        Ok(())
    }
}
