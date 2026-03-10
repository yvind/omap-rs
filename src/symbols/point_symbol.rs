use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{AreaSymbol, LineSymbol};
use crate::{
    Error, NonNegativeF64, Result,
    colors::{ColorSet, SymbolColor},
    objects::{AreaObject, LineObject, PointObject},
    symbols::SymbolCommon,
    utils::try_get_attr,
};

#[derive(Debug, Clone)]
pub enum Element {
    Point {
        symbol: Box<PointSymbol>,
        object: Box<PointObject>,
    },
    Line {
        symbol: Box<LineSymbol>,
        object: Box<LineObject>,
    },
    Area {
        symbol: Box<AreaSymbol>,
        object: Box<AreaObject>,
    },
}

impl Element {
    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>, color_set: &ColorSet) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("element")))?;
        match self {
            Element::Point { symbol, object } => {
                symbol.write(writer, color_set, None)?;
                object.write_as_element(writer)?;
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
    ) -> Result<Option<Element>> {
        let mut symbol_data: Option<ElementSymbolData> = None;
        let mut result = None;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    b"symbol" => {
                        let sym_type: u8 = try_get_attr(&e, "type").unwrap_or(0);
                        let mut sub_common = SymbolCommon::default();
                        for attr in e.attributes().filter_map(std::result::Result::ok) {
                            match attr.key.local_name().as_ref() {
                                b"name" => {
                                    sub_common.name.push_str(&quick_xml::escape::unescape(
                                        std::str::from_utf8(&attr.value)?,
                                    )?);
                                }
                                b"code" => {
                                    sub_common.code =
                                        crate::utils::parse_attr(attr.value).unwrap_or_default();
                                }
                                _ => {}
                            }
                        }
                        symbol_data = Some(match sym_type {
                            1 => ElementSymbolData::Point(Box::new(PointSymbol::parse(
                                reader, color_set, sub_common,
                            )?)),
                            2 => ElementSymbolData::Line(Box::new(LineSymbol::parse(
                                reader, color_set, sub_common,
                            )?)),
                            4 => ElementSymbolData::Area(Box::new(AreaSymbol::parse(
                                reader, color_set, sub_common,
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
                        if let Some(sym) = symbol_data.take() {
                            result = Some(Self::parse_element_object(reader, &e, sym)?);
                        }
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
        Ok(result)
    }

    fn parse_element_object<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        object_element: &BytesStart<'_>,
        sym: ElementSymbolData,
    ) -> Result<Element> {
        let rotation: f64 = try_get_attr(object_element, "rotation").unwrap_or(0.0);

        // Read through to find the <coords> element, then delegate
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => {
                    if e.local_name().as_ref() == b"coords" {
                        return match sym {
                            ElementSymbolData::Point(symbol) => {
                                let object = PointObject::parse(reader, &e, rotation)?;
                                Ok(Element::Point {
                                    symbol,
                                    object: Box::new(object),
                                })
                            }
                            ElementSymbolData::Line(symbol) => {
                                let object = LineObject::parse(reader, &e)?;
                                Ok(Element::Line {
                                    symbol,
                                    object: Box::new(object),
                                })
                            }
                            ElementSymbolData::Area(symbol) => {
                                let object = AreaObject::parse(reader, &e)?;
                                Ok(Element::Area {
                                    symbol,
                                    object: Box::new(object),
                                })
                            }
                        };
                    }
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == b"object" {
                        return Err(Error::ParseOmapFileError(
                            "Element object ended without coords".to_string(),
                        ));
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF parsing element object".to_string(),
                    ));
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PointSymbol {
    pub common: SymbolCommon,

    pub is_rotatable: bool,
    pub elements: Vec<Element>,

    pub inner_color: SymbolColor,
    pub outer_color: SymbolColor,
    pub inner_radius: NonNegativeF64,
    pub outer_width: NonNegativeF64,
}

/// Temporary enum used during element parsing
enum ElementSymbolData {
    Point(Box<PointSymbol>),
    Line(Box<LineSymbol>),
    Area(Box<AreaSymbol>),
}

impl PointSymbol {
    pub fn get_name(&self) -> &str {
        &self.common.name
    }
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<PointSymbol> {
        let mut common = attributes;
        let mut is_rotatable = false;
        let mut inner_radius = NonNegativeF64::default();
        let mut inner_color = SymbolColor::NoColor;
        let mut outer_width = NonNegativeF64::default();
        let mut outer_color = SymbolColor::NoColor;
        let mut elements = Vec::new();
        let mut _found_point_symbol = false;

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
                        _found_point_symbol = true;
                        is_rotatable = try_get_attr(&e, "rotatable").unwrap_or(false);
                        inner_radius = NonNegativeF64::from_file_value(
                            try_get_attr(&e, "inner_radius").unwrap_or(0),
                        );
                        let ic = try_get_attr(&e, "inner_color").unwrap_or(-1);
                        inner_color = SymbolColor::from_index(ic, color_set);
                        outer_width = NonNegativeF64::from_file_value(
                            try_get_attr(&e, "outer_width").unwrap_or(0),
                        );
                        let oc = try_get_attr(&e, "outer_color").unwrap_or(-1);
                        outer_color = SymbolColor::from_index(oc, color_set);
                    }
                    b"element" => {
                        if let Some(el) = Element::parse_element(reader, color_set)? {
                            elements.push(el);
                        }
                    }
                    _ => {}
                },
                Event::Empty(e) => {
                    if e.local_name().as_ref() == b"icon"
                        && let Some(src) = try_get_attr(&e, "src")
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
        let mut bs = BytesStart::new("symbol").with_attributes([
            ("type", "1"),
            ("code", self.common.code.to_string().as_str()),
            (
                "name",
                quick_xml::escape::unescape(self.common.name.as_str())?.as_ref(),
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
