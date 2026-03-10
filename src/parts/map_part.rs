use std::cell::RefCell;
use std::collections::HashMap;

use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::{Reader, Writer};

use crate::objects::MapObject;
use crate::symbols::{
    AreaSymbol, CombinedAreaSymbol, CombinedLineSymbol, LineSymbol, PointSymbol, SymbolSet,
    TextSymbol, WeakSymbol,
};
use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct MapPart {
    pub name: String,
    objects: HashMap<SymbolPointer, Vec<MapObject>>,
}

impl MapPart {
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
    pub(super) fn merge(&mut self, other: Self) {
        self.objects.extend(other.objects);
    }

    pub fn get(&self, key: &WeakSymbol) -> Option<&Vec<MapObject>> {
        self.objects.get(&key.into())
    }

    pub fn get_mut(&mut self, key: &WeakSymbol) -> Option<&mut Vec<MapObject>> {
        self.objects.get_mut(&key.into())
    }

    pub fn num_objects(&self) -> usize {
        self.objects.len()
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        symbols: &SymbolSet,
    ) -> Result<MapPart> {
        let mut name = String::new();

        for attr in element.attributes().filter_map(std::result::Result::ok) {
            if matches!(attr.key.local_name().as_ref(), b"name") {
                name = String::from_utf8(attr.value.to_vec())?;
            }
        }

        let mut objects: HashMap<SymbolPointer, Vec<MapObject>> = HashMap::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"object") {
                        let object = MapObject::parse(reader, &bytes_start, symbols)?;
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
