use std::rc::Weak;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{AreaSymbol, LineSymbol};
use crate::{
    Code, Error, NonNegativeF64, Result,
    colors::{ColorSet, SymbolColor},
    objects::{AreaObject, LineObject, PointObject},
    symbols::{SymbolCommon, WeakAreaPathSymbol, WeakLinePathSymbol},
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
    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>, color_set: &ColorSet) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("element")))?;
        match self {
            Element::Point { symbol, object } => {
                symbol.write(writer, color_set, None)?;
                object.write_as_element(writer, symbol.is_rotatable)?;
            }
            Element::Line { symbol, object } => {
                symbol.write(writer, color_set, None)?;
                object.write_as_element(writer)?;
            }
            Element::Area { symbol, object } => {
                symbol.write(writer, color_set, None)?;
                object.write_as_element(writer)?;
            }
        }
        writer.write_event(Event::End(BytesEnd::new("element")))?;
        Ok(())
    }

    /// Parse a single element inside point_symbol
    fn parse_element<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
    ) -> Result<Element> {
        let mut symbol_data = None;
        let mut object_data = None;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"symbol" => {
                        let sym_type = try_get_attr_raw(&e, "type").unwrap_or(0_u8);
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
                            _ => {
                                return Err(Error::ParseOmapFileError(format!(
                                    "Unknown element symbol type {sym_type}"
                                )));
                            }
                        });
                    }
                    b"object" => {
                        // Parse the object based on what symbol we have
                        let obj_type = try_get_attr_raw(&e, "type").unwrap_or(6_u8);
                        object_data = Some(match obj_type {
                            0 => ElementObjectData::Point(Box::new(PointObject::parse(
                                reader,
                                Weak::new(),
                                0.,
                            )?)),
                            1 => match &symbol_data {
                                Some(s) => match s {
                                    ElementSymbolData::Line(_) => {
                                        ElementObjectData::Line(Box::new(LineObject::parse(
                                            reader,
                                            WeakLinePathSymbol::Line(Weak::new()),
                                        )?))
                                    }
                                    ElementSymbolData::Area(_) => {
                                        ElementObjectData::Area(Box::new(AreaObject::parse(
                                            reader,
                                            WeakAreaPathSymbol::Area(Weak::new()),
                                        )?))
                                    }
                                    _ => {
                                        return Err(Error::ParseOmapFileError(
                                            "Symbol Object mismatch in element".to_string(),
                                        ));
                                    }
                                },
                                None => {
                                    return Err(Error::ParseOmapFileError(
                                        "Object before symbol in element".to_string(),
                                    ));
                                }
                            },
                            _ => {
                                return Err(Error::ParseOmapFileError(format!(
                                    "Unknown element object type {obj_type}"
                                )));
                            }
                        });
                    }
                    _ => {}
                },
                Event::End(e) => {
                    if e.local_name().as_ref() == b"element" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF parsing element".to_string(),
                    ));
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
                    return Ok(Element::Point {
                        symbol: point_symbol,
                        object: point_object,
                    });
                }
                (ElementSymbolData::Line(line_symbol), ElementObjectData::Line(line_object)) => {
                    return Ok(Element::Line {
                        symbol: line_symbol,
                        object: line_object,
                    });
                }
                (ElementSymbolData::Area(area_symbol), ElementObjectData::Area(area_object)) => {
                    return Ok(Element::Area {
                        symbol: area_symbol,
                        object: area_object,
                    });
                }
                _ => {
                    return Err(Error::ParseOmapFileError(
                        "Mismatch between object and symbol type in element".to_string(),
                    ));
                }
            }
        }
        Err(Error::ParseOmapFileError(
            "Either object or symbol data was not present in element".to_string(),
        ))
    }
}

/// A point symbol definition.
#[derive(Debug, Clone)]
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
    pub fn new(code: Code, name: String) -> PointSymbol {
        let common = SymbolCommon {
            code,
            name,
            ..Default::default()
        };
        PointSymbol {
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
    pub fn get_name(&self) -> &str {
        &self.common.name
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        mut common: SymbolCommon,
    ) -> Result<PointSymbol> {
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
                    b"description" => {
                        if let Event::Text(text) = reader.read_event_into(&mut buf)? {
                            common.description = String::from_utf8(text.to_vec())?;
                        }
                    }
                    b"point_symbol" => {
                        is_rotatable = try_get_attr_raw(&e, "rotatable").unwrap_or(is_rotatable);
                        inner_radius = NonNegativeF64::from_file_value(
                            try_get_attr_raw(&e, "inner_radius").unwrap_or(0),
                        );
                        inner_color = SymbolColor::from_index(
                            try_get_attr_raw(&e, "inner_color").unwrap_or(-1),
                            color_set,
                        );
                        outer_width = NonNegativeF64::from_file_value(
                            try_get_attr_raw(&e, "outer_width").unwrap_or(0),
                        );
                        outer_color = SymbolColor::from_index(
                            try_get_attr_raw(&e, "outer_color").unwrap_or(-1),
                            color_set,
                        );
                    }
                    b"element" => elements.push(Element::parse_element(reader, color_set)?),
                    _ => {}
                },
                Event::Empty(e) => {
                    if e.local_name().as_ref() == b"icon"
                        && let Some(src) = try_get_attr_raw(&e, "src")
                    {
                        common.custom_icon = Some(src);
                    }
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == b"symbol" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in PointSymbol parsing".to_string(),
                    ));
                }
                _ => {}
            }
        }

        // Check the point symbol for empty elements. Drop them
        let mut drop_elements = Vec::with_capacity(elements.len());
        for (i, element) in elements.iter().enumerate() {
            match element {
                Element::Point { symbol, object: _ } => {
                    if symbol.inner_color == SymbolColor::NoColor
                        && symbol.outer_color == SymbolColor::NoColor
                        && symbol.elements.is_empty()
                    {
                        drop_elements.push(i);
                    }
                }
                Element::Line { symbol: _, object } => {
                    if object.get_geometry().0.is_empty() {
                        drop_elements.push(i);
                    }
                }
                Element::Area { symbol: _, object } => {
                    if object.get_geometry().exterior().0.is_empty() {
                        drop_elements.push(i);
                    }
                }
            }
        }
        for i in drop_elements.into_iter().rev() {
            let _ = elements.swap_remove(i);
        }

        Ok(PointSymbol {
            common,
            is_rotatable,
            elements,
            inner_color,
            outer_color,
            inner_radius,
            outer_width,
        })
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
        index: Option<usize>,
    ) -> Result<()> {
        // Elements do not have codes so we should skip code writing for the default code
        let code_str = if self.common.code != Default::default() {
            self.common.code.to_string()
        } else {
            String::new()
        };

        let mut bs = BytesStart::new("symbol").with_attributes([
            ("type", "1"),
            ("code", code_str.as_str()),
            (
                "name",
                quick_xml::escape::escape(self.common.name.as_str()).as_ref(),
            ),
        ]);
        if let Some(id) = index {
            bs.push_attribute(("id", id.to_string().as_str()));
        }
        if self.common.is_hidden {
            bs.push_attribute(("is_hidden", "true"));
        }
        if self.common.is_helper_symbol {
            bs.push_attribute(("is_helper_symbol", "true"));
        }
        if self.common.is_protected {
            bs.push_attribute(("is_protected", "true"));
        }
        writer.write_event(Event::Start(bs))?;

        if !self.common.description.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("description")))?;
            writer.write_event(Event::Text(BytesText::new(&self.common.description)))?;
            writer.write_event(Event::End(BytesEnd::new("description")))?;
        }

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
            self.inner_color
                .get_priority(color_set)
                .to_string()
                .as_str(),
        ));
        bs.push_attribute((
            "outer_width",
            self.outer_width.to_file_value()?.to_string().as_str(),
        ));
        bs.push_attribute((
            "outer_color",
            self.outer_color
                .get_priority(color_set)
                .to_string()
                .as_str(),
        ));
        bs.push_attribute(("elements", self.elements.len().to_string().as_str()));
        writer.write_event(Event::Start(bs))?;

        for element in &self.elements {
            element.write(writer, color_set)?;
        }

        writer.write_event(Event::End(BytesEnd::new("point_symbol")))?;

        if let Some(icon) = &self.common.custom_icon {
            writer.write_event(Event::Empty(
                BytesStart::new("icon").with_attributes([("src", icon.as_str())]),
            ))?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbol")))?;
        Ok(())
    }
}
