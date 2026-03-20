use std::{cell::RefCell, collections::HashMap, rc::Weak};

use geo_types::{Coord, Point};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use crate::{
    Error, Result,
    symbols::{PointSymbol, Symbol, SymbolSet},
    utils::{from_file_coords, to_file_coords},
};

/// A point object placed at a single location on the map.
#[derive(Debug, Clone)]
pub struct PointObject {
    /// The tags associated with the object
    pub tags: HashMap<String, String>,
    /// Rotation of the symbol in radians.
    pub rotation: f64,
    /// Weak reference to the point symbol used to render this object.
    pub symbol: Weak<RefCell<PointSymbol>>,
    /// The point coordinates in mm on the map.
    pub geometry: Point,
}

impl PointObject {
    /// Create a new point object with the given symbol and position.
    pub fn new(symbol: Weak<RefCell<PointSymbol>>, geometry: Point) -> Self {
        PointObject {
            tags: HashMap::new(),
            rotation: 0.0,
            symbol,
            geometry,
        }
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
    ) -> Result<()> {
        let mut is_rotatable = false;
        // Get index of symbol and if the symbol is rotatable
        let index = if let Some(sym) = self.symbol.upgrade() {
            is_rotatable = sym.try_borrow().map(|p| p.is_rotatable).unwrap_or(false);
            symbol_set
                .iter()
                .position(|s| {
                    if let Symbol::Point(s) = s {
                        s.as_ptr() == sym.as_ptr()
                    } else {
                        false
                    }
                })
                .map(|p| p as i32)
                .unwrap_or(-1)
        } else {
            -1
        };

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
            // Map the rotation onto [-PI, PI]
            // first shift the target to either (-TAU, 0] for negative or [0, TAU) for positive
            // Take the modulus with TAU (negatives return negative values) and shift target back to [-PI, PI]
            let rot = (self.rotation + self.rotation.signum() * std::f64::consts::PI)
                % std::f64::consts::TAU
                - self.rotation.signum() * std::f64::consts::PI;
            bs.push_attribute(("rotation", rot.to_string().as_str()));
        }
        writer.write_event(Event::Start(bs))?;
        // elements are not allowed to have tags
        if !self.tags.is_empty() && symbol_index.is_some() {
            super::write_tags(writer, &self.tags)?;
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
        symbol: Weak<RefCell<PointSymbol>>,
        rotation: f64,
    ) -> Result<PointObject> {
        let mut tags = HashMap::new();
        let mut point = None;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"object") {
                        break;
                    }
                }
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"tags") {
                        tags = super::parse_tags(reader)?;
                    }
                }
                Event::Text(bytes_text) => {
                    let raw_xml = str::from_utf8(bytes_text.as_ref())?;

                    for vertex in raw_xml.split_terminator(';') {
                        let mut split = vertex.split_whitespace();

                        let x: i32 = split
                            .next()
                            .ok_or(Error::InvalidCoordinate("No x value".to_string()))?
                            .parse()?;
                        let y: i32 = split
                            .next()
                            .ok_or(Error::InvalidCoordinate("No y value".to_string()))?
                            .parse()?;
                        point = Some(Point::from(from_file_coords(Coord { x, y })));
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in PointObject parsing".to_string(),
                    ));
                }
                _ => (),
            }
        }
        Ok(PointObject {
            tags,
            rotation,
            symbol,
            geometry: point.ok_or(Error::ParseOmapFileError(
                "Could not parse point object".to_string(),
            ))?,
        })
    }
}
