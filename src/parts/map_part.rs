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
use crate::{Error, OmapSection, Result};

/// A map part (layer) containing objects grouped by symbol.
#[derive(Debug, Clone)]
pub struct MapPart {
    /// The name of this map part.
    pub name: String,
    objects: HashMap<SymbolPointer, Vec<MapObject>>,
}

impl MapPart {
    /// Create a new empty map part with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
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
            WeakSymbol::Line(weak) => Self::Line(weak.as_ptr()),
            WeakSymbol::Area(weak) => Self::Area(weak.as_ptr()),
            WeakSymbol::Point(weak) => Self::Point(weak.as_ptr()),
            WeakSymbol::Text(weak) => Self::Text(weak.as_ptr()),
            WeakSymbol::CombinedArea(weak) => Self::CombinedArea(weak.as_ptr()),
            WeakSymbol::CombinedLine(weak) => Self::CombinedLine(weak.as_ptr()),
        }
    }
}

impl MapPart {
    /// Add an object to the map
    pub fn add_object(&mut self, object: impl Into<MapObject>) {
        let mo = object.into();
        if mo.geometry_is_empty() {
            return;
        }

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
        for (p, object_vec) in other.objects {
            if let Some(contained_objects) = self.objects.get_mut(&p) {
                contained_objects.extend(object_vec);
            } else {
                let _ = self.objects.insert(p, object_vec);
            }
        }
    }

    /// Remove all objects with a symbol from the map
    pub fn remove(&mut self, key: &WeakSymbol) -> Option<Vec<MapObject>> {
        self.objects.remove(&key.into())
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
    pub fn num_symbols(&self) -> usize {
        self.objects.len()
    }

    /// Get the total number of objects in this part across all symbols.
    pub fn len(&self) -> usize {
        self.objects.values().map(|v| v.len()).sum()
    }

    /// Returns `true` if this part contains no objects.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Iterate through all objects in this map-part in a flat iterator.
    pub fn iter_all_objects(&self) -> impl Iterator<Item = &MapObject> {
        self.objects.values().flatten()
    }

    /// Iterate mutably through all objects in this map-part in a flat iterator.
    pub fn iter_all_objects_mut(&mut self) -> impl Iterator<Item = &mut MapObject> {
        self.objects.values_mut().flatten()
    }

    /// Iterate through the all the objects stored in this map-part, symbol by symbol
    pub fn iter(&self) -> impl Iterator<Item = &Vec<MapObject>> {
        self.objects.values()
    }

    /// Iterate mutabley through the all the objects stored in this map-part, symbol by symbol
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Vec<MapObject>> {
        self.objects.values_mut()
    }

    /// Consume this map-part and get all the objects it contains
    pub fn into_objects(self) -> Vec<MapObject> {
        self.objects.into_values().flatten().collect()
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
        symbols: &SymbolSet,
    ) -> Result<Self> {
        let name = try_get_attr(element, "name")
            .ok()
            .flatten()
            .unwrap_or(String::new());

        let mut objects: HashMap<SymbolPointer, Vec<MapObject>> = HashMap::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => {
                    if matches!(bytes_start.local_name().as_ref(), b"object") {
                        let object = MapObject::parse(reader, &bytes_start, symbols, false)?;
                        if object.geometry_is_empty() {
                            continue;
                        }

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
                    return Err(Error::UnexpectedEof(OmapSection::MapPart));
                }
                _ => (),
            }
        }

        Ok(Self { name, objects })
    }

    pub(super) fn write<W: std::io::Write>(
        self,
        writer: &mut Writer<W>,
        symbols: &SymbolSet,
    ) -> Result<()> {
        let object_count = self
            .objects
            .values()
            .flatten()
            .filter(|object| !object.geometry_is_empty())
            .count();

        writer.write_event(Event::Start(
            BytesStart::new("part")
                .with_attributes([("name", quick_xml::escape::escape(self.name.as_str()))]),
        ))?;
        writer.write_event(Event::Start(
            BytesStart::new("objects")
                .with_attributes([("count", object_count.to_string().as_str())]),
        ))?;

        for (_, objects) in self.objects {
            for object in objects {
                if object.geometry_is_empty() {
                    continue;
                }
                object.write(writer, symbols)?;
                writer.get_mut().write_all(b"\n")?;
            }
        }
        writer.write_event(Event::End(BytesEnd::new("objects")))?;
        writer.write_event(Event::End(BytesEnd::new("part")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Weak;

    use geo_types::{LineString, coord};
    use quick_xml::{Reader, Writer, events::Event};

    use super::MapPart;
    use crate::{
        Result,
        objects::{LineObject, MapObject},
        symbols::{SymbolSet, WeakLinePathSymbol},
    };

    fn empty_symbol_set() -> SymbolSet {
        SymbolSet {
            symbols: Vec::new(),
            name: String::new(),
        }
    }

    #[test]
    fn parsed_objects_without_geometry_are_ignored() -> Result<()> {
        let mut reader = Reader::from_str(
            r#"<part name="Map"><objects count="1"><object type="1"><coords count="0"></coords></object></objects></part>"#,
        );
        let Event::Start(start) = reader.read_event()? else {
            panic!("expected part start");
        };

        let part = MapPart::parse(&mut reader, &start, &empty_symbol_set())?;

        assert!(part.is_empty());
        Ok(())
    }

    #[test]
    fn empty_objects_are_rejected_on_insert_and_skipped_on_write() -> Result<()> {
        let weak_symbol = WeakLinePathSymbol::Line(Weak::new());
        let mut part = MapPart::new("Map");
        part.add_object(LineObject::new(
            weak_symbol.clone(),
            LineString::new(Vec::new()),
        ));
        assert!(part.is_empty());

        part.add_object(LineObject::new(
            weak_symbol,
            LineString::new(vec![coord! { x: 0., y: 0. }, coord! { x: 1., y: 0. }]),
        ));
        let Some(MapObject::Line(line)) = part.iter_all_objects_mut().next() else {
            panic!("expected line object");
        };
        line.get_geometry_mut(0.1)?.0.clear();

        let mut writer = Writer::new(Vec::new());
        part.write(&mut writer, &empty_symbol_set())?;
        let output = String::from_utf8(writer.into_inner())?;

        assert!(output.contains(r#"<objects count="0">"#));
        assert!(!output.contains("<object "));
        Ok(())
    }
}
