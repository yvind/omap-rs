use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use quick_xml::{Reader, Writer, events::BytesStart};

use super::{
    AreaSymbol, CombinedAreaSymbol, CombinedLineSymbol, LineSymbol, PointSymbol, SymbolSet,
    TextSymbol,
};
use crate::utils::{parse_attr, parse_attr_raw};
use crate::{Code, Error, Result, colors::ColorSet};

/// Common properties shared by all symbol types.
#[derive(Default, Debug, Clone)]
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

/// A non-owning reference to a symbol of any type.
#[derive(Debug, Clone)]
pub enum WeakSymbol {
    /// A weak reference to a line symbol.
    Line(Weak<RefCell<LineSymbol>>),
    /// A weak reference to an area symbol.
    Area(Weak<RefCell<AreaSymbol>>),
    /// A weak reference to a point symbol.
    Point(Weak<RefCell<PointSymbol>>),
    /// A weak reference to a text symbol.
    Text(Weak<RefCell<TextSymbol>>),
    /// A weak reference to a combined area symbol.
    CombinedArea(Weak<RefCell<CombinedAreaSymbol>>),
    /// A weak reference to a combined line symbol.
    CombinedLine(Weak<RefCell<CombinedLineSymbol>>),
}

impl WeakSymbol {
    /// Attempts to upgrade the WeakSymbol to a Symbol, delaying dropping of the inner value if successful.
    /// Returns None if the inner value has since been dropped.
    pub fn upgrade(&self) -> Option<Symbol> {
        match self {
            WeakSymbol::Line(weak) => weak.upgrade().map(Symbol::Line),
            WeakSymbol::Area(weak) => weak.upgrade().map(Symbol::Area),
            WeakSymbol::Point(weak) => weak.upgrade().map(Symbol::Point),
            WeakSymbol::Text(weak) => weak.upgrade().map(Symbol::Text),
            WeakSymbol::CombinedArea(weak) => weak.upgrade().map(Symbol::CombinedArea),
            WeakSymbol::CombinedLine(weak) => weak.upgrade().map(Symbol::CombinedLine),
        }
    }
}

/// An owning reference to a symbol of any type.
#[derive(Debug, Clone)]
pub enum Symbol {
    /// A line symbol.
    Line(Rc<RefCell<LineSymbol>>),
    /// An area symbol.
    Area(Rc<RefCell<AreaSymbol>>),
    /// A point symbol.
    Point(Rc<RefCell<PointSymbol>>),
    /// A text symbol.
    Text(Rc<RefCell<TextSymbol>>),
    /// Combined symbols can be either CombinedArea or CombinedLine
    /// The difference is what object geometry to relate with the symbol
    /// Mapper does not discern between any line and area objects
    CombinedArea(Rc<RefCell<CombinedAreaSymbol>>),
    /// A combined line symbol.
    CombinedLine(Rc<RefCell<CombinedLineSymbol>>),
}

impl Symbol {
    /// Creates a new WeakSymbol pointer to this Symbol allocation
    pub fn downgrade(&self) -> WeakSymbol {
        match self {
            Symbol::Line(rc) => WeakSymbol::Line(Rc::downgrade(rc)),
            Symbol::Area(rc) => WeakSymbol::Area(Rc::downgrade(rc)),
            Symbol::Point(rc) => WeakSymbol::Point(Rc::downgrade(rc)),
            Symbol::Text(rc) => WeakSymbol::Text(Rc::downgrade(rc)),
            Symbol::CombinedArea(rc) => WeakSymbol::CombinedArea(Rc::downgrade(rc)),
            Symbol::CombinedLine(rc) => WeakSymbol::CombinedLine(Rc::downgrade(rc)),
        }
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Line(l0), Self::Line(r0)) => l0.as_ptr() == r0.as_ptr(),
            (Self::Area(l0), Self::Area(r0)) => l0.as_ptr() == r0.as_ptr(),
            (Self::Point(l0), Self::Point(r0)) => l0.as_ptr() == r0.as_ptr(),
            (Self::Text(l0), Self::Text(r0)) => l0.as_ptr() == r0.as_ptr(),
            (Self::CombinedArea(l0), Self::CombinedArea(r0)) => l0.as_ptr() == r0.as_ptr(),
            (Self::CombinedLine(l0), Self::CombinedLine(r0)) => l0.as_ptr() == r0.as_ptr(),
            _ => false,
        }
    }
}

macro_rules! impl_symbol_getter {
    ($method:ident -> $ret_type:ty, |$s:ident| $expr:expr) => {
        /// Only fails if the symbol's ref_cell cannot be borrowed, i.e. the symbol's ref_cell is mutably borrowed somewhere else
        pub fn $method(&self) -> Result<$ret_type> {
            match self {
                Symbol::Line(rc) => {
                    let $s = rc.try_borrow()?;
                    Ok($expr)
                }
                Symbol::Area(rc) => {
                    let $s = rc.try_borrow()?;
                    Ok($expr)
                }
                Symbol::Point(rc) => {
                    let $s = rc.try_borrow()?;
                    Ok($expr)
                }
                Symbol::Text(rc) => {
                    let $s = rc.try_borrow()?;
                    Ok($expr)
                }
                Symbol::CombinedLine(rc) => {
                    let $s = rc.try_borrow()?;
                    Ok($expr)
                }
                Symbol::CombinedArea(rc) => {
                    let $s = rc.try_borrow()?;
                    Ok($expr)
                }
            }
        }
    };
}
macro_rules! impl_symbol_setter {
    ($method:ident($param:ident: $param_type:ty), |$s:ident| $expr:expr) => {
        /// Only fails if the symbol's ref_cell cannot be mutably borrowed, i.e. the symbol's ref_cell is borrowed somewhere else
        pub fn $method(&self, $param: $param_type) -> Result<()> {
            match self {
                Symbol::Line(rc) => {
                    let mut $s = rc.try_borrow_mut()?;
                    $expr
                }
                Symbol::Area(rc) => {
                    let mut $s = rc.try_borrow_mut()?;
                    $expr
                }
                Symbol::Point(rc) => {
                    let mut $s = rc.try_borrow_mut()?;
                    $expr
                }
                Symbol::Text(rc) => {
                    let mut $s = rc.try_borrow_mut()?;
                    $expr
                }
                Symbol::CombinedLine(rc) => {
                    let mut $s = rc.try_borrow_mut()?;
                    $expr
                }
                Symbol::CombinedArea(rc) => {
                    let mut $s = rc.try_borrow_mut()?;
                    $expr
                }
            }
            Ok(())
        }
    };
}

impl Symbol {
    impl_symbol_getter!(has_custom_icon -> bool, |s| s.common.custom_icon.is_some());
    impl_symbol_setter!(set_custom_icon(icon: Option<String>), |s| s.common.custom_icon = icon);
    impl_symbol_getter!(get_code -> Code, |s| s.common.code);
    impl_symbol_setter!(set_code(code: Code), |s| s.common.code = code);
    impl_symbol_getter!(is_helper_symbol -> bool, |s| s.common.is_helper_symbol);
    impl_symbol_setter!(set_helper_symbol(is_helper: bool), |s| s.common.is_helper_symbol = is_helper);
    impl_symbol_getter!(is_hidden -> bool, |s| s.common.is_helper_symbol);
    impl_symbol_setter!(set_hidden(is_hidden: bool), |s| s.common.is_hidden = is_hidden);
    impl_symbol_getter!(is_protected -> bool, |s| s.common.is_helper_symbol);
    impl_symbol_setter!(set_protected(is_protected: bool), |s| s.common.is_protected = is_protected);

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        color_set: &ColorSet,
    ) -> Result<(usize, Symbol, Vec<usize>)> {
        let mut id = usize::MAX;
        let mut symbol_type = u8::MAX;
        let mut common = SymbolCommon::default();
        // Parse attributes
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
            return Err(Error::ParseOmapFileError(
                "Could not parse symbol".to_string(),
            ));
        }

        // We must record the component IDs for combined symbols
        // and parse them after all symbols have been parsed
        let mut public_component_ids = Vec::new();
        let symbol = match symbol_type {
            1 => Symbol::Point(Rc::new(RefCell::new(PointSymbol::parse(
                reader, color_set, common,
            )?))),
            2 => Symbol::Line(Rc::new(RefCell::new(LineSymbol::parse(
                reader, color_set, common,
            )?))),
            4 => Symbol::Area(Rc::new(RefCell::new(AreaSymbol::parse(
                reader, color_set, common,
            )?))),
            8 => Symbol::Text(Rc::new(RefCell::new(TextSymbol::parse(
                reader, color_set, common,
            )?))),
            16 => {
                // Assume the combined symbol is area for now
                // Will be checked and corrected after all symbols have been parsed
                let (symbol, component_ids) = CombinedAreaSymbol::parse(reader, color_set, common)?;
                public_component_ids.extend(component_ids);

                Symbol::CombinedArea(Rc::new(RefCell::new(symbol)))
            }
            _ => {
                return Err(Error::ParseOmapFileError(format!(
                    "Could not parse symbol of type {symbol_type}"
                )));
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
        match self {
            // Line, area and point can be sub-symbols which do not have an index
            Symbol::Line(rc) => rc.try_borrow()?.write(writer, color_set, Some(index)),
            Symbol::Area(rc) => rc.try_borrow()?.write(writer, color_set, Some(index)),
            Symbol::Point(rc) => rc.try_borrow()?.write(writer, color_set, Some(index)),
            Symbol::Text(rc) => rc.try_borrow()?.write(writer, color_set, index),
            Symbol::CombinedArea(rc) => {
                rc.try_borrow()?.write(writer, symbol_set, color_set, index)
            }
            Symbol::CombinedLine(rc) => {
                rc.try_borrow()?.write(writer, symbol_set, color_set, index)
            }
        }?;
        Ok(())
    }
}
