use std::{
    cell::{Ref, RefCell, RefMut},
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
    /// Attempts to upgrade the `WeakSymbol` to a Symbol, delaying dropping of the inner value if successful.
    /// Returns None if the inner value has since been dropped.
    pub fn upgrade(&self) -> Option<Symbol> {
        match self {
            Self::Line(weak) => weak.upgrade().map(Symbol::Line),
            Self::Area(weak) => weak.upgrade().map(Symbol::Area),
            Self::Point(weak) => weak.upgrade().map(Symbol::Point),
            Self::Text(weak) => weak.upgrade().map(Symbol::Text),
            Self::CombinedArea(weak) => weak.upgrade().map(Symbol::CombinedArea),
            Self::CombinedLine(weak) => weak.upgrade().map(Symbol::CombinedLine),
        }
    }
}

impl PartialEq for WeakSymbol {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Line(l0), Self::Line(r0)) => l0.ptr_eq(r0),
            (Self::Area(l0), Self::Area(r0)) => l0.ptr_eq(r0),
            (Self::Point(l0), Self::Point(r0)) => l0.ptr_eq(r0),
            (Self::Text(l0), Self::Text(r0)) => l0.ptr_eq(r0),
            (Self::CombinedArea(l0), Self::CombinedArea(r0)) => l0.ptr_eq(r0),
            (Self::CombinedLine(l0), Self::CombinedLine(r0)) => l0.ptr_eq(r0),
            _ => false,
        }
    }
}

macro_rules! impl_from_weak_symbol {
    ($symbol_ty:ty, $variant:ident) => {
        impl From<Weak<RefCell<$symbol_ty>>> for WeakSymbol {
            fn from(value: Weak<RefCell<$symbol_ty>>) -> Self {
                WeakSymbol::$variant(value)
            }
        }
    };
}

impl_from_weak_symbol!(AreaSymbol, Area);
impl_from_weak_symbol!(LineSymbol, Line);
impl_from_weak_symbol!(PointSymbol, Point);
impl_from_weak_symbol!(TextSymbol, Text);
impl_from_weak_symbol!(CombinedAreaSymbol, CombinedArea);
impl_from_weak_symbol!(CombinedLineSymbol, CombinedLine);

macro_rules! impl_try_from_weak_symbol {
    ($symbol_ty:ty, $variant:ident) => {
        impl TryFrom<WeakSymbol> for Weak<RefCell<$symbol_ty>> {
            type Error = Error;

            fn try_from(value: WeakSymbol) -> std::result::Result<Self, Self::Error> {
                if let WeakSymbol::$variant(weak) = value {
                    Ok(weak)
                } else {
                    Err(Error::SymbolConversionError)
                }
            }
        }
    };
}

impl_try_from_weak_symbol!(AreaSymbol, Area);
impl_try_from_weak_symbol!(LineSymbol, Line);
impl_try_from_weak_symbol!(PointSymbol, Point);
impl_try_from_weak_symbol!(TextSymbol, Text);
impl_try_from_weak_symbol!(CombinedAreaSymbol, CombinedArea);
impl_try_from_weak_symbol!(CombinedLineSymbol, CombinedLine);

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
    /// Combined symbols can be either `CombinedArea` or `CombinedLine`
    /// The difference is what object geometry to relate with the symbol
    /// Mapper does not discern between any line and area objects
    CombinedArea(Rc<RefCell<CombinedAreaSymbol>>),
    /// A combined line symbol.
    CombinedLine(Rc<RefCell<CombinedLineSymbol>>),
}

impl Symbol {
    /// Creates a new `WeakSymbol` pointer to this Symbol allocation
    pub fn downgrade(&self) -> WeakSymbol {
        match self {
            Self::Line(rc) => WeakSymbol::Line(Rc::downgrade(rc)),
            Self::Area(rc) => WeakSymbol::Area(Rc::downgrade(rc)),
            Self::Point(rc) => WeakSymbol::Point(Rc::downgrade(rc)),
            Self::Text(rc) => WeakSymbol::Text(Rc::downgrade(rc)),
            Self::CombinedArea(rc) => WeakSymbol::CombinedArea(Rc::downgrade(rc)),
            Self::CombinedLine(rc) => WeakSymbol::CombinedLine(Rc::downgrade(rc)),
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

macro_rules! impl_from_symbol {
    ($symbol_ty:ty, $variant:ident) => {
        impl From<$symbol_ty> for Symbol {
            fn from(value: $symbol_ty) -> Self {
                Symbol::$variant(Rc::new(RefCell::new(value)))
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
        /// Access a common symbol property.
        ///
        /// Takes its own borrow; use [`Symbol::common`] to read several
        /// properties of the same symbol at once.
        ///
        /// # Errors
        ///
        /// Returns an error if the symbol's `RefCell` is currently mutably
        /// borrowed.
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
        /// Update a common symbol property.
        ///
        /// Takes its own borrow; use [`Symbol::common_mut`] to update several
        /// properties of the same symbol at once.
        ///
        /// # Errors
        ///
        /// Returns an error if the symbol's `RefCell` is already borrowed.
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
    /// Borrow the [`SymbolCommon`] properties shared by every symbol type.
    ///
    /// The individual getters below take one `RefCell` borrow per call and
    /// clone the `String` fields. Reading several properties of the same
    /// symbol — filtering on [`SymbolCommon::is_helper_symbol`] and
    /// [`SymbolCommon::is_hidden`] before dispatching on
    /// [`SymbolCommon::code`], say — is a single borrow through this accessor,
    /// and the strings can be read by reference.
    ///
    /// The returned guard keeps the symbol immutably borrowed; the setters and
    /// anything else needing a mutable borrow will fail while it is alive.
    ///
    /// # Errors
    ///
    /// Returns an error if the symbol's `RefCell` is currently mutably
    /// borrowed.
    pub fn common(&self) -> Result<Ref<'_, SymbolCommon>> {
        Ok(match self {
            Self::Line(rc) => Ref::map(rc.try_borrow()?, |s| &s.common),
            Self::Area(rc) => Ref::map(rc.try_borrow()?, |s| &s.common),
            Self::Point(rc) => Ref::map(rc.try_borrow()?, |s| &s.common),
            Self::Text(rc) => Ref::map(rc.try_borrow()?, |s| &s.common),
            Self::CombinedLine(rc) => Ref::map(rc.try_borrow()?, |s| &s.common),
            Self::CombinedArea(rc) => Ref::map(rc.try_borrow()?, |s| &s.common),
        })
    }

    /// Mutably borrow the [`SymbolCommon`] properties shared by every symbol
    /// type.
    ///
    /// The counterpart of [`Symbol::common`] for the setters below, collapsing
    /// a run of updates to the same symbol into a single borrow.
    ///
    /// The returned guard keeps the symbol mutably borrowed; every other
    /// accessor will fail while it is alive.
    ///
    /// # Errors
    ///
    /// Returns an error if the symbol's `RefCell` is already borrowed.
    pub fn common_mut(&self) -> Result<RefMut<'_, SymbolCommon>> {
        Ok(match self {
            Self::Line(rc) => RefMut::map(rc.try_borrow_mut()?, |s| &mut s.common),
            Self::Area(rc) => RefMut::map(rc.try_borrow_mut()?, |s| &mut s.common),
            Self::Point(rc) => RefMut::map(rc.try_borrow_mut()?, |s| &mut s.common),
            Self::Text(rc) => RefMut::map(rc.try_borrow_mut()?, |s| &mut s.common),
            Self::CombinedLine(rc) => RefMut::map(rc.try_borrow_mut()?, |s| &mut s.common),
            Self::CombinedArea(rc) => RefMut::map(rc.try_borrow_mut()?, |s| &mut s.common),
        })
    }

    impl_symbol_getter!(has_custom_icon -> bool, |s| s.common.custom_icon.is_some());
    impl_symbol_setter!(set_custom_icon(icon: Option<String>), |s| s.common.custom_icon = icon);
    impl_symbol_getter!(get_code -> Code, |s| s.common.code);
    impl_symbol_setter!(set_code(code: Code), |s| s.common.code = code);
    impl_symbol_getter!(is_helper_symbol -> bool, |s| s.common.is_helper_symbol);
    impl_symbol_setter!(set_helper_symbol(is_helper: bool), |s| s.common.is_helper_symbol = is_helper);
    impl_symbol_getter!(is_hidden -> bool, |s| s.common.is_hidden);
    impl_symbol_setter!(set_hidden(is_hidden: bool), |s| s.common.is_hidden = is_hidden);
    impl_symbol_getter!(is_protected -> bool, |s| s.common.is_protected);
    impl_symbol_setter!(set_protected(is_protected: bool), |s| s.common.is_protected = is_protected);
    impl_symbol_getter!(get_name -> String, |s| s.common.name.clone());
    impl_symbol_setter!(set_name(name: String), |s| s.common.name = name);
    impl_symbol_getter!(get_description -> String, |s| s.common.description.clone());
    impl_symbol_setter!(set_description(description: String), |s| s.common.description = description);

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        color_set: &ColorSet,
    ) -> Result<(usize, Self, Vec<usize>)> {
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
            return Err(Error::MissingSymbolId);
        }

        // We must record the component IDs for combined symbols
        // and parse them after all symbols have been parsed
        let mut public_component_ids = Vec::new();
        let symbol = match symbol_type {
            1 => Self::Point(Rc::new(RefCell::new(PointSymbol::parse(
                reader, color_set, common,
            )?))),
            2 => Self::Line(Rc::new(RefCell::new(LineSymbol::parse(
                reader, color_set, common,
            )?))),
            4 => Self::Area(Rc::new(RefCell::new(AreaSymbol::parse(
                reader, color_set, common,
            )?))),
            8 => Self::Text(Rc::new(RefCell::new(TextSymbol::parse(
                reader, color_set, common,
            )?))),
            16 => {
                // Assume the combined symbol is area for now
                // Will be checked and corrected after all symbols have been parsed
                let (symbol, component_ids) = CombinedAreaSymbol::parse(reader, color_set, common)?;
                public_component_ids.extend(component_ids);

                Self::CombinedArea(Rc::new(RefCell::new(symbol)))
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
        match self {
            // Line, area and point can be sub-symbols which do not have an index
            Self::Line(rc) => rc.try_borrow()?.write(writer, color_set, Some(index)),
            Self::Area(rc) => rc.try_borrow()?.write(writer, color_set, Some(index)),
            Self::Point(rc) => rc.try_borrow()?.write(writer, color_set, Some(index)),
            Self::Text(rc) => rc.try_borrow()?.write(writer, color_set, index),
            Self::CombinedArea(rc) => rc.try_borrow()?.write(writer, symbol_set, color_set, index),
            Self::CombinedLine(rc) => rc.try_borrow()?.write(writer, symbol_set, color_set, index),
        }?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Symbol;
    use crate::{
        Code, Result,
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
    fn common_sees_the_same_values_as_the_individual_getters() -> Result<()> {
        for symbol in one_of_each() {
            symbol.set_helper_symbol(true)?;
            symbol.set_description("a description".to_owned())?;

            let common = symbol.common()?;
            assert_eq!(common.code, symbol.get_code()?, "code mismatch");
            assert_eq!(common.name, symbol.get_name()?, "name mismatch");
            assert_eq!(common.description, "a description", "description mismatch");
            assert!(common.is_helper_symbol, "helper flag mismatch");
            assert!(!common.is_hidden, "hidden flag mismatch");
        }
        Ok(())
    }

    #[test]
    fn common_mut_writes_through_to_the_getters() -> Result<()> {
        for symbol in one_of_each() {
            {
                let mut common = symbol.common_mut()?;
                common.name = "renamed".to_owned();
                common.code = Code::new(4, 5, 6);
                common.is_hidden = true;
            }

            assert_eq!(symbol.get_name()?, "renamed");
            assert_eq!(symbol.get_code()?, Code::new(4, 5, 6));
            assert!(symbol.is_hidden()?);
        }
        Ok(())
    }

    #[test]
    fn borrows_conflict_as_a_refcell_does() -> Result<()> {
        for symbol in one_of_each() {
            let common = symbol.common()?;
            assert!(
                symbol.set_hidden(true).is_err(),
                "a setter must not succeed while a shared guard is alive"
            );
            assert!(
                symbol.common_mut().is_err(),
                "common_mut must not succeed while a shared guard is alive"
            );
            assert!(
                symbol.is_hidden().is_ok(),
                "a getter must still succeed while a shared guard is alive"
            );
            drop(common);

            let common = symbol.common_mut()?;
            assert!(
                symbol.is_hidden().is_err(),
                "a getter must not succeed while an exclusive guard is alive"
            );
            drop(common);

            assert!(symbol.common().is_ok(), "guards must release on drop");
        }
        Ok(())
    }
}
