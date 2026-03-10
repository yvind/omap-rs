mod template;
mod transform;

pub use template::Template;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};

use crate::{
    Error, NonNegativeF64, Result,
    utils::{parse_attr, try_get_attr},
    view::TemplateVisibility,
};

#[derive(Debug, Clone)]
pub struct TemplateDefaults {
    pub use_meters_per_pixel: bool,
    pub meters_per_pixel: NonNegativeF64,
    pub dpi: NonNegativeF64,
    pub scale: u32,
}

impl Default for TemplateDefaults {
    fn default() -> Self {
        Self {
            use_meters_per_pixel: true,
            meters_per_pixel: NonNegativeF64::default(),
            dpi: NonNegativeF64::default(),
            scale: 0,
        }
    }
}

impl TemplateDefaults {
    fn parse_attrs(bs: &BytesStart<'_>) -> Self {
        let mut d = Self::default();
        for attr in bs.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"use_meters_per_pixel" => {
                    d.use_meters_per_pixel = attr.as_bool().unwrap_or(d.use_meters_per_pixel)
                }
                b"meters_per_pixel" => {
                    d.meters_per_pixel = parse_attr(attr.value)
                        .unwrap_or(d.meters_per_pixel.get())
                        .try_into()
                        .unwrap_or_default()
                }
                b"dpi" => {
                    d.dpi = parse_attr(attr.value)
                        .unwrap_or(d.dpi.get())
                        .try_into()
                        .unwrap_or_default()
                }
                b"scale" => d.scale = parse_attr(attr.value).unwrap_or(d.scale),
                _ => {}
            }
        }
        d
    }

    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Empty(BytesStart::new("defaults").with_attributes([
            (
                "use_meters_per_pixel",
                self.use_meters_per_pixel.to_string().as_str(),
            ),
            (
                "meters_per_pixel",
                format!("{:.2}", self.meters_per_pixel.get()).as_str(),
            ),
            ("dpi", format!("{:.2}", self.dpi.get()).as_str()),
            ("scale", self.scale.to_string().as_str()),
        ])))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TemplateEntry {
    pub template: Template,
    pub visibilty: TemplateVisibility,
}

impl TemplateEntry {
    fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<TemplateVisibility> {
        self.template.write(writer)?;
        Ok(self.visibilty)
    }
}

/// All templates attached to the map, plus default display settings.
#[derive(Debug, Default, Clone)]
pub struct Templates {
    /// The template entries, ordered back-to-front.
    /// A [TemplateEntry] is a [Template] and [TemplateVisibility]
    pub template_entries: Vec<TemplateEntry>,
    /// Index of the first [Template] that is drawn in front of the map.
    /// Templates with `index >= first_front_template` are in front of the map.
    pub first_front_template: u32,
    /// Default rendering parameters shown in the template setup dialog.
    pub defaults: TemplateDefaults,
}

impl Templates {
    /// Number of templates.
    pub fn len(&self) -> usize {
        self.template_entries.len()
    }

    /// Whether there are no templates.
    pub fn is_empty(&self) -> bool {
        self.template_entries.is_empty()
    }

    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        event: &BytesStart<'_>,
    ) -> Result<Self> {
        let first_front_template = try_get_attr(event, "first_front_template").unwrap_or(0);

        let mut templates = Vec::new();
        let mut defaults = TemplateDefaults::default();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bs) => match bs.local_name().as_ref() {
                    b"template" => {
                        templates.push(Template::parse(reader, &bs)?);
                    }
                    b"defaults" => {
                        defaults = TemplateDefaults::parse_attrs(&bs);
                    }
                    _ => {}
                },
                Event::End(be) if be.local_name().as_ref() == b"templates" => break,
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in templates".into(),
                    ));
                }
                _ => {}
            }
        }

        let template_entries = templates
            .into_iter()
            .map(|t| TemplateEntry {
                template: t,
                visibilty: TemplateVisibility::default(),
            })
            .collect();

        Ok(Templates {
            template_entries,
            first_front_template,
            defaults,
        })
    }

    pub(crate) fn write<W: std::io::Write>(
        self,
        writer: &mut Writer<W>,
    ) -> Result<Vec<TemplateVisibility>> {
        writer.write_event(Event::Start(BytesStart::new("templates").with_attributes(
            [
                ("count", self.template_entries.len().to_string().as_str()),
                (
                    "first_front_template",
                    self.first_front_template.to_string().as_str(),
                ),
            ],
        )))?;

        let mut visibilities = Vec::with_capacity(self.len());
        for entry in self.template_entries {
            visibilities.push(entry.write(writer)?);
        }

        self.defaults.write(writer)?;

        writer.write_event(Event::End(BytesEnd::new("templates")))?;
        Ok(visibilities)
    }
}
