use std::collections::HashMap;

use quick_xml::events::BytesStart;

use crate::editor::objects::MapObject;
use crate::editor::symbols::SymbolId;
use crate::editor::{Result, Transform};

#[derive(Debug, Clone)]
pub struct MapPart {
    pub name: String,
    pub objects: HashMap<SymbolId, Vec<MapObject>>,
}

impl MapPart {
    pub(super) fn merge(&mut self, other: Self) {
        self.objects.extend(other.objects);
    }
}

impl MapPart {
    pub(super) fn parse_part(element: &BytesStart) -> Result<MapPart> {
        let mut name = String::new();

        // Parse attributes
        for attr in element.attributes() {
            let attr = attr?;
            let key = std::str::from_utf8(attr.key.as_ref())?;
            let value = std::str::from_utf8(&attr.value)?;

            if key == "name" {
                name = value.to_string();
            }
        }

        Ok(MapPart {
            name,
            objects: HashMap::new(),
        })
    }

    pub(super) fn write<W: std::io::Write>(
        self,
        writer: &mut W,
        transform: &Transform,
    ) -> Result<()> {
        writer.write_all(
            format!(
                "<part name=\"{}\"><objects count=\"{}\">\n",
                self.name,
                self.objects.len()
            )
            .as_bytes(),
        )?;

        for (_, objects) in self.objects {
            for object in objects {
                object.write(writer, transform)?;
            }
        }

        writer.write_all("</objects></part>\n".as_bytes())?;
        Ok(())
    }
}
