use std::cell::RefCell;
use std::collections::HashMap;

use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::{Reader, Writer};

use crate::objects::MapObject;
use crate::symbols::{
    AreaSymbol, CombinedAreaSymbol, CombinedLineSymbol, LineSymbol, PointSymbol, SymbolSet,
    TextSymbol, WeakAreaPathSymbol, WeakLinePathSymbol, WeakSymbol,
};
use crate::utils::try_get_attr;
use crate::{Error, Result};

/// A map part (layer) containing objects grouped by symbol.
#[derive(Debug, Clone)]
pub struct MapPart {
    /// The name of this map part.
    pub name: String,
    objects: HashMap<SymbolPointer, Vec<MapObject>>,
}

impl MapPart {
    /// Create a new empty map part with the given name.
    pub fn new(name: impl Into<String>) -> MapPart {
        MapPart {
            name: name.into(),
            objects: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum SymbolPointer {
    Point(*const RefCell<PointSymbol>),
    Line(*const RefCell<LineSymbol>),
    Area(*const RefCell<AreaSymbol>),
    Text(*const RefCell<TextSymbol>),
    CombinedArea(*const RefCell<CombinedAreaSymbol>),
    CombinedLine(*const RefCell<CombinedLineSymbol>),
}

impl From<&WeakSymbol> for SymbolPointer {
    fn from(value: &WeakSymbol) -> Self {
        match value {
            WeakSymbol::Line(weak) => SymbolPointer::Line(weak.as_ptr()),
            WeakSymbol::Area(weak) => SymbolPointer::Area(weak.as_ptr()),
            WeakSymbol::Point(weak) => SymbolPointer::Point(weak.as_ptr()),
            WeakSymbol::Text(weak) => SymbolPointer::Text(weak.as_ptr()),
            WeakSymbol::CombinedArea(weak) => SymbolPointer::CombinedArea(weak.as_ptr()),
            WeakSymbol::CombinedLine(weak) => SymbolPointer::CombinedLine(weak.as_ptr()),
        }
    }
}

impl MapPart {
    /// Add an object to the map
    pub fn add_object(&mut self, object: impl Into<MapObject>) {
        let mo = object.into();
        let pointer = match &mo {
            MapObject::Point(o) => SymbolPointer::Point(o.symbol.as_ptr()),
            MapObject::Line(o) => match &o.symbol {
                WeakLinePathSymbol::Line(weak) => SymbolPointer::Line(weak.as_ptr()),
                WeakLinePathSymbol::CombinedLine(weak) => {
                    SymbolPointer::CombinedLine(weak.as_ptr())
                }
            },
            MapObject::Area(o) => match &o.symbol {
                WeakAreaPathSymbol::Area(weak) => SymbolPointer::Area(weak.as_ptr()),
                WeakAreaPathSymbol::CombinedArea(weak) => {
                    SymbolPointer::CombinedArea(weak.as_ptr())
                }
            },
            MapObject::Text(o) => SymbolPointer::Text(o.symbol.as_ptr()),
        };

        if let Some(values) = self.objects.get_mut(&pointer) {
            values.push(mo);
        } else {
            let _ = self.objects.insert(pointer, vec![mo]);
        }
    }

    pub(super) fn merge(&mut self, other: Self) {
        self.objects.extend(other.objects);
    }

    /// Get objects associated with a symbol.
    pub fn get(&self, key: &WeakSymbol) -> Option<&Vec<MapObject>> {
        self.objects.get(&key.into())
    }

    /// Get a mutable reference to objects associated with a symbol.
    pub fn get_mut(&mut self, key: &WeakSymbol) -> Option<&mut Vec<MapObject>> {
        self.objects.get_mut(&key.into())
    }

    /// Get the number of distinct symbols with objects in this part.
    pub fn num_objects(&self) -> usize {
        self.objects.len()
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        symbols: &SymbolSet,
    ) -> Result<MapPart> {
        let name = try_get_attr(element, "name").unwrap_or(String::new());

        let mut objects: HashMap<SymbolPointer, Vec<MapObject>> = HashMap::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"object") {
                        let object = MapObject::parse(reader, &bytes_start, symbols, false)?;
                        let symbol_pointer = (&object.get_weak_symbol()).into();

                        if let Some(contained) = objects.get_mut(&symbol_pointer) {
                            contained.push(object);
                        } else {
                            let _ = objects.insert(symbol_pointer, vec![object]);
                        }
                    }
                }
                Event::End(bytes_end) => {
                    if matches!(bytes_end.local_name().as_ref(), b"part") {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in parsing MapPart".to_string(),
                    ));
                }
                _ => (),
            }
        }

        Ok(MapPart { name, objects })
    }

    pub(super) fn write<W: std::io::Write>(
        self,
        writer: &mut Writer<W>,
        symbols: &SymbolSet,
    ) -> Result<()> {
        writer.write_event(Event::Start(
            BytesStart::new("part")
                .with_attributes([("name", quick_xml::escape::escape(self.name.as_str()))]),
        ))?;
        writer
            .write_event(Event::Start(BytesStart::new("objects").with_attributes([
                ("count", self.objects.len().to_string().as_str()),
            ])))?;

        for (_, objects) in self.objects {
            for object in objects {
                object.write(writer, symbols)?;
            }
        }

        writer.write_event(Event::End(BytesEnd::new("objects")))?;
        writer.write_event(Event::End(BytesEnd::new("part")))?;
        Ok(())
    }
}
