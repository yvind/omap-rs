use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use super::{AreaSymbol, LineSymbol, SymbolKind, symbol::SymbolPosition};
use crate::{
    Code, Error, NonNegativeF64, OmapSection, Result,
    colors::{ColorId, ColorSet, SymbolColor},
    notes,
    objects::{AreaObject, LineObject, PointObject},
    symbols::SymbolCommon,
    utils::try_get_attr_raw,
};

/// Temporary enum used during element parsing
enum ElementSymbolData {
    Point(Box<PointSymbol>),
    Line(Box<LineSymbol>),
    Area(Box<AreaSymbol>),
}

/// Temporary enum used during element parsing
enum ElementObjectData {
    Point(Box<PointObject>),
    Line(Box<LineObject>),
    Area(Box<AreaObject>),
}

/// An element within a point symbol definition.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Element {
    /// A nested point sub-symbol with its object.
    Point {
        /// The point sub-symbol.
        symbol: Box<PointSymbol>,
        /// The object rendered by this element.
        object: Box<PointObject>,
    },
    /// A line sub-symbol with its object.
    Line {
        /// The line sub-symbol.
        symbol: Box<LineSymbol>,
        /// The object rendered by this element.
        object: Box<LineObject>,
    },
    /// An area sub-symbol with its object.
    Area {
        /// The area sub-symbol.
        symbol: Box<AreaSymbol>,
        /// The object rendered by this element.
        object: Box<AreaObject>,
    },
}

impl Element {
    fn is_empty(&self) -> bool {
        match self {
            Self::Point { symbol, .. } => {
                symbol.inner_color == SymbolColor::NoColor
                    && symbol.outer_color == SymbolColor::NoColor
                    && symbol.elements.is_empty()
            }
            Self::Line { object, .. } => object.geometry_is_empty(),
            Self::Area { object, .. } => object.geometry_is_empty(),
        }
    }

    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>, color_set: &ColorSet) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        writer.write_event(Event::Start(BytesStart::new("element")))?;
        match self {
            Self::Point { symbol, object } => {
                symbol.write(writer, color_set, SymbolPosition::Element)?;
                object.write_as_element(writer, symbol.is_rotatable)?;
            }
            Self::Line { symbol, object } => {
                symbol.write(writer, color_set, SymbolPosition::Element)?;
                object.write_as_element(writer)?;
            }
            Self::Area { symbol, object } => {
                symbol.write(writer, color_set, SymbolPosition::Element)?;
                object.write_as_element(writer)?;
            }
        }
        writer.write_event(Event::End(BytesEnd::new("element")))?;
        Ok(())
    }

    /// Parse a single element inside `point_symbol`
    fn parse_element<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
    ) -> Result<Self> {
        let mut symbol_data = None;
        let mut object_data = None;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"symbol" => {
                        let sym_type = try_get_attr_raw(&e, "type")?.unwrap_or(0_u8);
                        symbol_data = Some(match sym_type {
                            1 => ElementSymbolData::Point(Box::new(PointSymbol::parse(
                                reader,
                                color_set,
                                Default::default(),
                            )?)),
                            2 => ElementSymbolData::Line(Box::new(LineSymbol::parse(
                                reader,
                                color_set,
                                Default::default(),
                            )?)),
                            4 => ElementSymbolData::Area(Box::new(AreaSymbol::parse(
                                reader,
                                color_set,
                                Default::default(),
                            )?)),
                            _ => return Err(Error::UnknownElementSymbolType(sym_type)),
                        });
                    }
                    b"object" => {
                        let obj_type = try_get_attr_raw(&e, "type")?.unwrap_or(6_u8);
                        object_data = Some(match obj_type {
                            0 => ElementObjectData::Point(Box::new(PointObject::parse(
                                reader, None, 0.,
                            )?)),
                            1 => match &symbol_data {
                                Some(s) => match s {
                                    ElementSymbolData::Line(_) => ElementObjectData::Line(
                                        Box::new(LineObject::parse(reader, None)?),
                                    ),
                                    ElementSymbolData::Area(_) => ElementObjectData::Area(
                                        Box::new(AreaObject::parse(reader, None)?),
                                    ),
                                    ElementSymbolData::Point(_) => {
                                        return Err(Error::ElementSymbolObjectMismatch);
                                    }
                                },
                                None => return Err(Error::ElementObjectBeforeSymbol),
                            },
                            _ => return Err(Error::UnknownElementObjectType(obj_type)),
                        });
                    }
                    _ => {}
                },
                Event::End(e) if e.local_name().as_ref() == b"element" => {
                    break;
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::Element));
                }
                _ => {}
            }
        }
        if let Some(sd) = symbol_data
            && let Some(od) = object_data
        {
            match (sd, od) {
                (
                    ElementSymbolData::Point(point_symbol),
                    ElementObjectData::Point(point_object),
                ) => {
                    return Ok(Self::Point {
                        symbol: point_symbol,
                        object: point_object,
                    });
                }
                (ElementSymbolData::Line(line_symbol), ElementObjectData::Line(line_object)) => {
                    return Ok(Self::Line {
                        symbol: line_symbol,
                        object: line_object,
                    });
                }
                (ElementSymbolData::Area(area_symbol), ElementObjectData::Area(area_object)) => {
                    return Ok(Self::Area {
                        symbol: area_symbol,
                        object: area_object,
                    });
                }
                _ => {
                    return Err(Error::ElementSymbolObjectMismatch);
                }
            }
        }
        Err(Error::MissingElementData)
    }
}

/// A point symbol definition.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PointSymbol {
    /// Common symbol properties.
    pub common: SymbolCommon,

    /// Whether the symbol is rotatable.
    pub is_rotatable: bool,
    /// The graphical elements that make up this symbol.
    pub elements: Vec<Element>,

    /// Inner circle colour.
    pub inner_color: SymbolColor,
    /// Outer ring colour.
    pub outer_color: SymbolColor,
    /// Inner circle radius in mm.
    pub inner_radius: NonNegativeF64,
    /// Outer ring width in mm.
    pub outer_width: NonNegativeF64,
}

impl PointSymbol {
    /// Create a new empty point symbol with the given code and name.
    pub fn new(code: Code, name: impl Into<String>) -> Self {
        let common = SymbolCommon {
            code,
            name: name.into(),
            ..Default::default()
        };
        Self {
            common,
            is_rotatable: true,
            elements: Vec::new(),
            inner_color: SymbolColor::NoColor,
            outer_color: SymbolColor::NoColor,
            inner_radius: NonNegativeF64::default(),
            outer_width: NonNegativeF64::default(),
        }
    }

    /// Get the display name of this point symbol.
    pub fn name(&self) -> &str {
        &self.common.name
    }

    /// Set the inner circle colour (builder-style).
    pub fn with_inner_color(mut self, color: SymbolColor) -> Self {
        self.inner_color = color;
        self
    }

    /// Set the outer ring colour (builder-style).
    pub fn with_outer_color(mut self, color: SymbolColor) -> Self {
        self.outer_color = color;
        self
    }

    /// Set the inner circle radius in mm (builder-style).
    pub fn with_inner_radius(mut self, radius: NonNegativeF64) -> Self {
        self.inner_radius = radius;
        self
    }

    /// Set the outer ring width in mm (builder-style).
    pub fn with_outer_width(mut self, width: NonNegativeF64) -> Self {
        self.outer_width = width;
        self
    }

    /// Add a graphical element (builder-style).
    pub fn with_element(mut self, element: Element) -> Self {
        if !element.is_empty() {
            self.elements.push(element);
        }
        self
    }

    /// Set whether the symbol is rotatable (builder-style).
    pub fn with_rotatable(mut self, rotatable: bool) -> Self {
        self.is_rotatable = rotatable;
        self
    }

    /// Mark as a helper symbol (builder-style).
    pub fn as_helper_symbol(mut self) -> Self {
        self.common.is_helper_symbol = true;
        self
    }

    pub fn colors(&self) -> Vec<ColorId> {
        let mut colors = Vec::new();

        if let SymbolColor::Color(id) = &self.inner_color {
            colors.push(*id);
        }
        if let SymbolColor::Color(id) = &self.outer_color {
            colors.push(*id);
        }

        for element in &self.elements {
            colors.extend(match element {
                Element::Point { symbol, object: _ } => symbol.colors(),
                Element::Line { symbol, object: _ } => symbol.colors(),
                Element::Area { symbol, object: _ } => symbol.colors(),
            });
        }

        colors
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        mut common: SymbolCommon,
    ) -> Result<Self> {
        let mut is_rotatable = false;
        let mut inner_radius = NonNegativeF64::default();
        let mut inner_color = SymbolColor::NoColor;
        let mut outer_width = NonNegativeF64::default();
        let mut outer_color = SymbolColor::NoColor;
        let mut elements = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"description" => common.description = notes::parse(reader)?,
                    b"point_symbol" => {
                        is_rotatable = try_get_attr_raw(&e, "rotatable")?.unwrap_or(is_rotatable);
                        inner_radius = NonNegativeF64::from_file_value(
                            try_get_attr_raw(&e, "inner_radius")?.unwrap_or(0),
                        );
                        inner_color = SymbolColor::from_index(
                            try_get_attr_raw(&e, "inner_color")?.unwrap_or(-1),
                            color_set,
                        );
                        outer_width = NonNegativeF64::from_file_value(
                            try_get_attr_raw(&e, "outer_width")?.unwrap_or(0),
                        );
                        outer_color = SymbolColor::from_index(
                            try_get_attr_raw(&e, "outer_color")?.unwrap_or(-1),
                            color_set,
                        );
                    }
                    b"element" => elements.push(Element::parse_element(reader, color_set)?),
                    b"icon" => common.custom_icon = try_get_attr_raw(&e, "src")?,
                    _ => {}
                },
                Event::End(e) if e.local_name().as_ref() == b"symbol" => {
                    break;
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::PointSymbol));
                }
                _ => {}
            }
        }

        elements.retain(|element| !element.is_empty());

        Ok(Self {
            common,
            is_rotatable,
            elements,
            inner_color,
            outer_color,
            inner_radius,
            outer_width,
        })
    }

    /// Write this symbol on its own, for the sub-symbol positions that
    /// [`Symbol::write`] does not reach: private parts of a combined symbol,
    /// and point-symbol elements.
    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
        position: SymbolPosition,
    ) -> Result<()> {
        self.common
            .write_open(writer, SymbolKind::Point.type_id(), position)?;
        self.write_body(writer, color_set)?;
        self.common.write_close(writer)
    }

    /// Write the type-specific body, between the halves of the shared
    /// `<symbol>` frame written by [`Symbol::write`].
    pub(super) fn write_body<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
    ) -> Result<()> {
        let mut bs = BytesStart::new("point_symbol");
        if self.is_rotatable {
            bs.push_attribute(("rotatable", "true"));
        }
        bs.push_attribute((
            "inner_radius",
            self.inner_radius.to_file_value()?.to_string().as_str(),
        ));
        bs.push_attribute((
            "inner_color",
            self.inner_color.priority(color_set).to_string().as_str(),
        ));
        bs.push_attribute((
            "outer_width",
            self.outer_width.to_file_value()?.to_string().as_str(),
        ));
        bs.push_attribute((
            "outer_color",
            self.outer_color.priority(color_set).to_string().as_str(),
        ));
        let element_count = self
            .elements
            .iter()
            .filter(|element| !element.is_empty())
            .count();
        bs.push_attribute(("elements", element_count.to_string().as_str()));
        writer.write_event(Event::Start(bs))?;

        for element in self.elements.iter().filter(|element| !element.is_empty()) {
            element.write(writer, color_set)?;
        }

        writer.write_event(Event::End(BytesEnd::new("point_symbol")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use geo_types::LineString;
    use quick_xml::Writer;

    use super::{Element, PointSymbol, SymbolPosition};
    use crate::{Code, Result, colors::ColorSet, objects::LineObject, symbols::LineSymbol};

    fn empty_line_element() -> Element {
        Element::Line {
            symbol: Box::new(LineSymbol::new(Code::default(), "")),
            object: Box::new(LineObject::new_element(LineString::new(Vec::new()))),
        }
    }

    #[test]
    fn empty_path_elements_are_rejected_and_skipped_on_write() -> Result<()> {
        let symbol = PointSymbol::new(Code::default(), "").with_element(empty_line_element());
        assert!(symbol.elements.is_empty());

        let mut symbol = PointSymbol::new(Code::default(), "");
        symbol.elements.push(empty_line_element());
        let mut writer = Writer::new(Vec::new());
        symbol.write(&mut writer, &ColorSet::default(), SymbolPosition::Private)?;
        let output = String::from_utf8(writer.into_inner())?;

        assert!(output.contains(r#"elements="0""#));
        assert!(!output.contains("<element>"));
        Ok(())
    }
}
