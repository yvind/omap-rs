use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::{
    AreaSymbol, CombinedAreaSymbol, CombinedLineSymbol, LineSymbol, PointSymbol, SymbolKind,
    SymbolSet, TextSymbol,
};

use crate::{
    Code, Error, Result,
    colors::{ColorId, ColorSet},
    utils::{parse_attr, parse_attr_raw},
};

/// Where a `<symbol>` element sits in the file, which decides whether its
/// opening tag carries an index and a code.
#[derive(Clone, Copy)]
pub(super) enum SymbolPosition {
    /// A symbol in the set: index and code.
    Indexed(usize),
    /// A private sub-symbol of a combined symbol: code only.
    Private,
    /// A point-symbol element: neither.
    Element,
}

/// Common properties shared by all symbol types.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolCommon {
    /// The symbol's name
    pub name: String,
    /// The symbol's code, of the form A.B.C
    pub code: Code,
    /// A description of the symbol
    pub description: String,
    /// Do not show the symbol on the printed map
    pub is_helper_symbol: bool,
    /// Hide the symbol in oomapper
    pub is_hidden: bool,
    /// Protect the symbol in oomapper
    pub is_protected: bool,
    /// base64 encoded symbol icon
    pub custom_icon: Option<String>,
}

impl SymbolCommon {
    /// Write the `<symbol>` opening tag and the `<description>` that follows it.
    pub(super) fn write_open<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_type: &str,
        position: SymbolPosition,
    ) -> Result<()> {
        let code = match position {
            SymbolPosition::Element => String::new(),
            SymbolPosition::Indexed(_) | SymbolPosition::Private => self.code.to_string(),
        };
        let mut bs = BytesStart::new("symbol").with_attributes([
            ("type", symbol_type),
            ("code", code.as_str()),
            ("name", self.name.as_str()),
        ]);
        if let SymbolPosition::Indexed(index) = position {
            bs.push_attribute(("id", index.to_string().as_str()));
        }
        if self.is_hidden {
            bs.push_attribute(("is_hidden", "true"));
        }
        if self.is_helper_symbol {
            bs.push_attribute(("is_helper_symbol", "true"));
        }
        if self.is_protected {
            bs.push_attribute(("is_protected", "true"));
        }
        writer.write_event(Event::Start(bs))?;

        if !self.description.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("description")))?;
            writer.write_event(Event::Text(BytesText::new(&self.description)))?;
            writer.write_event(Event::End(BytesEnd::new("description")))?;
        }
        Ok(())
    }

    /// Write the `<icon>` and close the `<symbol>`.
    pub(super) fn write_close<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        if let Some(icon) = &self.custom_icon {
            writer.write_event(Event::Empty(
                BytesStart::new("icon").with_attributes([("src", icon.as_str())]),
            ))?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbol")))?;
        Ok(())
    }
}

/// A symbol of any type.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Symbol {
    /// A line symbol.
    Line(Box<LineSymbol>),
    /// An area symbol.
    Area(Box<AreaSymbol>),
    /// A point symbol.
    Point(Box<PointSymbol>),
    /// A text symbol.
    Text(Box<TextSymbol>),
    /// Combined symbols can be either `CombinedArea` or `CombinedLine`
    /// The difference is what object geometry to relate with the symbol
    /// Mapper does not discern between any line and area objects
    CombinedArea(Box<CombinedAreaSymbol>),
    /// A combined line symbol.
    CombinedLine(Box<CombinedLineSymbol>),
}

macro_rules! impl_from_symbol {
    ($symbol_ty:ty, $variant:ident) => {
        impl From<$symbol_ty> for Symbol {
            fn from(value: $symbol_ty) -> Self {
                Symbol::$variant(Box::new(value))
            }
        }
    };
}

impl_from_symbol!(AreaSymbol, Area);
impl_from_symbol!(LineSymbol, Line);
impl_from_symbol!(PointSymbol, Point);
impl_from_symbol!(TextSymbol, Text);
impl_from_symbol!(CombinedAreaSymbol, CombinedArea);
impl_from_symbol!(CombinedLineSymbol, CombinedLine);

macro_rules! impl_symbol_getter {
    ($method:ident -> $ret_type:ty, |$s:ident| $expr:expr) => {
        /// Read a property shared by every symbol type.
        pub fn $method(&self) -> $ret_type {
            let $s = self.common();
            $expr
        }
    };
}

macro_rules! impl_symbol_setter {
    ($method:ident($param:ident: $param_type:ty), |$s:ident| $expr:expr) => {
        /// Update a property shared by every symbol type.
        pub fn $method(&mut self, $param: $param_type) {
            let $s = self.common_mut();
            $expr;
        }
    };
}

impl Symbol {
    /// The [`SymbolCommon`] properties shared by every symbol type.
    pub fn common(&self) -> &SymbolCommon {
        match self {
            Self::Line(symbol) => &symbol.common,
            Self::Area(symbol) => &symbol.common,
            Self::Point(symbol) => &symbol.common,
            Self::Text(symbol) => &symbol.common,
            Self::CombinedLine(symbol) => &symbol.common,
            Self::CombinedArea(symbol) => &symbol.common,
        }
    }

    /// The [`SymbolCommon`] properties shared by every symbol type, mutably.
    pub fn common_mut(&mut self) -> &mut SymbolCommon {
        match self {
            Self::Line(symbol) => &mut symbol.common,
            Self::Area(symbol) => &mut symbol.common,
            Self::Point(symbol) => &mut symbol.common,
            Self::Text(symbol) => &mut symbol.common,
            Self::CombinedLine(symbol) => &mut symbol.common,
            Self::CombinedArea(symbol) => &mut symbol.common,
        }
    }

    /// Every color used in this symbol definition.
    ///
    /// Takes the [`SymbolSet`] that owns the symbol, because a combined
    /// symbol's public components live in the set rather than in the symbol.
    /// A component no longer in the set contributes no colors.
    pub fn colors(&self, symbol_set: &SymbolSet) -> Vec<ColorId> {
        match self {
            Self::Line(symbol) => symbol.colors(),
            Self::Area(symbol) => symbol.colors(),
            Self::Point(symbol) => symbol.colors(),
            Self::Text(symbol) => symbol.colors(),
            Self::CombinedArea(symbol) => symbol.colors(symbol_set),
            Self::CombinedLine(symbol) => symbol.colors(symbol_set),
        }
    }

    impl_symbol_getter!(custom_icon -> Option<&str>, |s| s.custom_icon.as_deref());
    impl_symbol_getter!(has_custom_icon -> bool, |s| s.custom_icon.is_some());
    impl_symbol_setter!(set_custom_icon(icon: Option<String>), |s| s.custom_icon = icon);
    impl_symbol_getter!(code -> Code, |s| s.code);
    impl_symbol_setter!(set_code(code: Code), |s| s.code = code);
    impl_symbol_getter!(is_helper_symbol -> bool, |s| s.is_helper_symbol);
    impl_symbol_setter!(set_helper_symbol(is_helper: bool), |s| s.is_helper_symbol = is_helper);
    impl_symbol_getter!(is_hidden -> bool, |s| s.is_hidden);
    impl_symbol_setter!(set_hidden(is_hidden: bool), |s| s.is_hidden = is_hidden);
    impl_symbol_getter!(is_protected -> bool, |s| s.is_protected);
    impl_symbol_setter!(set_protected(is_protected: bool), |s| s.is_protected = is_protected);
    impl_symbol_getter!(name -> &str, |s| s.name.as_str());
    impl_symbol_setter!(set_name(name: String), |s| s.name = name);
    impl_symbol_getter!(description -> &str, |s| s.description.as_str());
    impl_symbol_setter!(set_description(description: String), |s| s.description = description);

    /// The kind of this symbol, which is also the kind of any handle to it.
    pub fn kind(&self) -> SymbolKind {
        match self {
            Self::Line(_) => SymbolKind::Line,
            Self::Area(_) => SymbolKind::Area,
            Self::Point(_) => SymbolKind::Point,
            Self::Text(_) => SymbolKind::Text,
            Self::CombinedArea(_) => SymbolKind::CombinedArea,
            Self::CombinedLine(_) => SymbolKind::CombinedLine,
        }
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        color_set: &ColorSet,
    ) -> Result<(usize, Self, Vec<usize>)> {
        let mut id = usize::MAX;
        let mut symbol_type = u8::MAX;
        let mut common = SymbolCommon::default();
        for attr in element.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"type" => symbol_type = parse_attr_raw(attr.value).unwrap_or(symbol_type),
                b"name" => common.name = parse_attr(attr, element.decoder()).unwrap_or(common.name),
                b"code" => common.code = parse_attr_raw(attr.value).unwrap_or(common.code),
                b"id" => id = parse_attr_raw(attr.value).unwrap_or(id),
                b"is_helper_symbol" => {
                    common.is_helper_symbol = attr.as_bool().unwrap_or(false);
                }
                b"is_hidden" => {
                    common.is_hidden = attr.as_bool().unwrap_or(false);
                }
                b"is_protected" => {
                    common.is_protected = attr.as_bool().unwrap_or(false);
                }
                _ => {}
            }
        }

        if id == usize::MAX {
            return Err(Error::MissingSymbolId);
        }

        // Components can only be resolved once every symbol is parsed.
        let mut public_component_ids = Vec::new();
        let symbol = match symbol_type {
            1 => Self::Point(Box::new(PointSymbol::parse(reader, color_set, common)?)),
            2 => Self::Line(Box::new(LineSymbol::parse(reader, color_set, common)?)),
            4 => Self::Area(Box::new(AreaSymbol::parse(reader, color_set, common)?)),
            8 => Self::Text(Box::new(TextSymbol::parse(reader, color_set, common)?)),
            16 => {
                // Reclassified as a line symbol later if its components say so.
                let (symbol, component_ids) = CombinedAreaSymbol::parse(reader, color_set, common)?;
                public_component_ids.extend(component_ids);

                Self::CombinedArea(Box::new(symbol))
            }
            _ => {
                return Err(Error::UnknownSymbolType(symbol_type));
            }
        };

        Ok((id, symbol, public_component_ids))
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        symbol_set: &SymbolSet,
        color_set: &ColorSet,
        index: usize,
    ) -> Result<()> {
        let common = self.common();
        common.write_open(
            writer,
            self.kind().type_id(),
            SymbolPosition::Indexed(index),
        )?;
        match self {
            Self::Line(symbol) => symbol.write_body(writer, color_set),
            Self::Area(symbol) => symbol.write_body(writer, color_set),
            Self::Point(symbol) => symbol.write_body(writer, color_set),
            Self::Text(symbol) => symbol.write_body(writer, color_set),
            Self::CombinedArea(symbol) => symbol.write_body(writer, symbol_set, color_set),
            Self::CombinedLine(symbol) => symbol.write_body(writer, symbol_set, color_set),
        }?;
        common.write_close(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::Symbol;
    use crate::{
        Code,
        symbols::{
            AreaSymbol, CombinedAreaSymbol, CombinedLineSymbol, LineSymbol, PointSymbol, TextSymbol,
        },
    };

    fn one_of_each() -> Vec<Symbol> {
        let code = Code::new(1, 2, 3);
        vec![
            LineSymbol::new(code, "line").into(),
            AreaSymbol::new(code, "area").into(),
            PointSymbol::new(code, "point").into(),
            TextSymbol::new(code, "text").into(),
            CombinedAreaSymbol::new(code, "combined area").into(),
            CombinedLineSymbol::new(code, "combined line").into(),
        ]
    }

    #[test]
    fn common_sees_the_same_values_as_the_individual_getters() {
        for mut symbol in one_of_each() {
            symbol.set_helper_symbol(true);
            symbol.set_description("a description".to_owned());

            let common = symbol.common();
            assert_eq!(common.code, symbol.code(), "code mismatch");
            assert_eq!(common.name, symbol.name(), "name mismatch");
            assert_eq!(common.description, "a description", "description mismatch");
            assert!(common.is_helper_symbol, "helper flag mismatch");
            assert!(!common.is_hidden, "hidden flag mismatch");
        }
    }

    #[test]
    fn common_mut_writes_through_to_the_getters() {
        for mut symbol in one_of_each() {
            {
                let common = symbol.common_mut();
                common.name = "renamed".to_owned();
                common.code = Code::new(4, 5, 6);
                common.is_hidden = true;
            }

            assert_eq!(symbol.name(), "renamed");
            assert_eq!(symbol.code(), Code::new(4, 5, 6));
            assert!(symbol.is_hidden());
        }
    }
}
