use std::collections::HashMap;

use geo_types::{Coord, Point};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use crate::{
    CoordinateComponent, Error, ObjectKind, OmapSection, Result,
    symbols::{PointSymbolId, SymbolSet},
    utils::{from_file_coords, to_file_coords, transform_position, try_transform_position},
};

/// A point object placed at a single location on the map.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PointObject {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    #[expect(
        clippy::box_collection,
        reason = "the map header is 48 bytes inline and most objects carry no tags"
    )]
    tags: Option<Box<HashMap<String, String>>>,
    /// Rotation of the symbol in radians.
    pub rotation: f64,
    /// The point symbol used to render this object, or `None` for the
    /// format's unknown-symbol sentinel.
    pub symbol: Option<PointSymbolId>,
    geometry: Point,
}

impl PointObject {
    /// Create a new point object with the given symbol and position.
    pub fn new(symbol: Option<PointSymbolId>, geometry: Point) -> Self {
        Self {
            tags: None,
            rotation: 0.0,
            symbol,
            geometry,
        }
    }

    /// Create a point object for use as a point-symbol element.
    pub fn new_element(geometry: Point) -> Self {
        Self::new(None, geometry)
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

    /// Get a shared reference to the point geometry.
    pub fn geometry(&self) -> &Point {
        &self.geometry
    }

    /// Get a mutable reference to the point geometry.
    pub fn geometry_mut(&mut self) -> &mut Point {
        &mut self.geometry
    }

    /// Transform the point position and symbol rotation.
    pub fn transform<F>(&mut self, transform: F)
    where
        F: Fn(Coord) -> Coord,
    {
        let (position, rotation, _) = transform_position(self.geometry.0, transform);
        self.geometry.0 = position;
        self.rotation += rotation;
    }

    /// Try to transform the point position and symbol rotation.
    ///
    /// # Errors
    ///
    /// Returns any error produced by `transform`. The object is unchanged on
    /// failure.
    pub fn try_transform<E, F>(&mut self, transform: F) -> std::result::Result<(), E>
    where
        F: Fn(Coord) -> std::result::Result<Coord, E>,
    {
        let (position, rotation, _) = try_transform_position(self.geometry.0, transform)?;
        self.geometry.0 = position;
        self.rotation += rotation;
        Ok(())
    }

    /// Consume this object and return its geometry.
    pub fn into_geometry(self) -> Point {
        self.geometry
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let is_rotatable = self
            .symbol
            .and_then(|id| symbol_set.point_symbol(id))
            .is_some_and(|symbol| symbol.is_rotatable);
        let index = symbol_set.file_index(self.symbol);

        self.write_content(writer, Some(index), is_rotatable)?;
        Ok(())
    }

    /// Write a full `<object>...</object>` element - used for point symbol elements
    pub(crate) fn write_as_element<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        is_rotatable: bool,
    ) -> Result<()> {
        self.write_content(writer, None, is_rotatable)?;
        Ok(())
    }

    fn write_content<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_index: Option<i32>,
        is_rotatable: bool,
    ) -> Result<()> {
        let mut bs = BytesStart::new("object").with_attributes([("type", "0")]);
        if let Some(idx) = symbol_index {
            bs.push_attribute(("symbol", idx.to_string().as_str()));
        }

        if self.rotation.abs() > f64::EPSILON && is_rotatable {
            // Map the rotation onto [-PI, PI].
            let rot = (self.rotation + self.rotation.signum() * std::f64::consts::PI)
                % std::f64::consts::TAU
                - self.rotation.signum() * std::f64::consts::PI;
            bs.push_attribute(("rotation", rot.to_string().as_str()));
        }
        writer.write_event(Event::Start(bs))?;
        // elements are not allowed to have tags
        if !self.tags().is_empty() && symbol_index.is_some() {
            super::write_tags(writer, self.tags())?;
        }
        let file_coord = to_file_coords(self.geometry.0)?;
        writer.write_event(Event::Start(
            BytesStart::new("coords").with_attributes([("count", "1")]),
        ))?;
        writer.write_event(Event::Text(BytesText::new(&format!(
            "{} {};",
            file_coord.x, file_coord.y
        ))))?;
        writer.write_event(Event::End(BytesEnd::new("coords")))?;
        writer.write_event(Event::End(BytesEnd::new("object")))?;
        Ok(())
    }

    /// Parse a point object. The reader should be positioned right after
    /// the `<coords>` start event. Reads through `</object>`.
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        symbol: Option<PointSymbolId>,
        rotation: f64,
    ) -> Result<Self> {
        let mut tags = None;
        let mut point = None;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), "object") {
                        break;
                    }
                }
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), "tags") {
                        tags = super::parse_tags(reader)?;
                    }
                }
                Event::Text(bytes_text) => {
                    let raw_xml = bytes_text.as_ref();

                    for vertex in raw_xml.split_terminator(';') {
                        let mut split = vertex.split_whitespace();

                        let x: i32 = split
                            .next()
                            .ok_or(Error::MissingCoordinateComponent(CoordinateComponent::X))?
                            .parse()?;
                        let y: i32 = split
                            .next()
                            .ok_or(Error::MissingCoordinateComponent(CoordinateComponent::Y))?
                            .parse()?;
                        point = Some(Point::from(from_file_coords(Coord { x, y })));
                    }
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::PointObject));
                }
                _ => (),
            }
        }
        Ok(Self {
            tags,
            rotation,
            symbol,
            geometry: point.ok_or(Error::MissingObjectGeometry(ObjectKind::Point))?,
        })
    }
}
