use std::path::PathBuf;

use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::transform::{AdjustmentState, Transformations};
use crate::{
    Error, Result,
    templates::transform::{PassPoint, TemplateTransform},
    utils::parse_attr,
};

/// A template attached to the map. Each variant carries type-specific data.
#[derive(Debug, Clone)]
pub enum Template {
    /// A raster image template.
    Image(ImageTemplate),
    /// A map file template.
    Map(MapTemplate),
    /// A GPS track template.
    Track(TrackTemplate),
    /// A geospatial raster data template via GDAL.
    Gdal(GdalTemplate),
    /// A geospatial vector data template via OGR.
    Ogr(OgrTemplate),
}

/// A raster image template.
#[derive(Debug, Clone)]
pub struct ImageTemplate {
    pub common: TemplateCommon,
}

impl ImageTemplate {
    fn write<W: std::io::Write>(&self, _writer: &mut Writer<W>) -> Result<()> {
        Ok(())
    }
}

/// A map file template.
#[derive(Debug, Clone)]
pub struct MapTemplate {
    pub common: TemplateCommon,
}

impl MapTemplate {
    fn write<W: std::io::Write>(&self, _writer: &mut Writer<W>) -> Result<()> {
        Ok(())
    }
}

/// A GPS track template.
#[derive(Debug, Clone)]
pub struct TrackTemplate {
    pub common: TemplateCommon,
    pub track_crs_spec: String,
    pub projected_crs_spec: String,
}

impl TrackTemplate {
    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        if !self.track_crs_spec.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("track_crs_spec")))?;
            writer.write_event(Event::Text(BytesText::new(&self.track_crs_spec)))?;
            writer.write_event(Event::End(BytesEnd::new("track_crs_spec")))?;
        }
        if !self.projected_crs_spec.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("projected_crs_spec")))?;
            writer.write_event(Event::Text(BytesText::new(&self.projected_crs_spec)))?;
            writer.write_event(Event::End(BytesEnd::new("projected_crs_spec")))?;
        }
        Ok(())
    }
}

/// A geospatial raster data template (via GDAL).
#[derive(Debug, Clone)]
pub struct GdalTemplate {
    pub common: TemplateCommon,
    pub crs_spec: String,
}

impl GdalTemplate {
    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("crs_spec")))?;
        writer.write_event(Event::Text(BytesText::new(&self.crs_spec)))?;
        writer.write_event(Event::End(BytesEnd::new("crs_spec")))?;
        Ok(())
    }
}

/// A geospatial vector data template (via OGR).
#[derive(Debug, Clone)]
pub struct OgrTemplate {
    pub common: TemplateCommon,
    pub crs_spec: String,
    pub track_crs_spec: String,
    pub projected_crs_spec: String,
}

impl OgrTemplate {
    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        if !self.crs_spec.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("crs_spec")))?;
            writer.write_event(Event::Text(BytesText::new(&self.crs_spec)))?;
            writer.write_event(Event::End(BytesEnd::new("crs_spec")))?;
        }
        if !self.track_crs_spec.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("track_crs_spec")))?;
            writer.write_event(Event::Text(BytesText::new(&self.track_crs_spec)))?;
            writer.write_event(Event::End(BytesEnd::new("track_crs_spec")))?;
        }
        if !self.projected_crs_spec.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("projected_crs_spec")))?;
            writer.write_event(Event::Text(BytesText::new(&self.projected_crs_spec)))?;
            writer.write_event(Event::End(BytesEnd::new("projected_crs_spec")))?;
        }
        Ok(())
    }
}

impl Template {
    pub fn get_common(&self) -> &TemplateCommon {
        match self {
            Template::Image(t) => &t.common,
            Template::Map(t) => &t.common,
            Template::Track(t) => &t.common,
            Template::Gdal(t) => &t.common,
            Template::Ogr(t) => &t.common,
        }
    }

    /// Returns the template type name as used in the XML format.
    pub fn type_name(&self) -> &'static str {
        match self {
            Template::Image(_) => "TemplateImage",
            Template::Map(_) => "TemplateMap",
            Template::Track(_) => "TemplateTrack",
            Template::Gdal(_) => "GdalTemplate",
            Template::Ogr(_) => "OgrTemplate",
        }
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        bs: &BytesStart<'_>,
    ) -> Result<Self> {
        let mut template_type = String::new();
        let mut is_open = false;
        let mut name = String::new();
        let mut path = PathBuf::new();
        let mut relpath = PathBuf::new();
        let mut is_georeferenced = false;
        let mut group = -1;

        for attr in bs.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"type" => template_type = parse_attr(attr.value).unwrap_or(template_type),
                b"open" => is_open = attr.as_bool().unwrap_or(false),
                b"name" => name = parse_attr(attr.value).unwrap_or(name),
                b"path" => path = parse_attr(attr.value).unwrap_or(path),
                b"relpath" => relpath = parse_attr(attr.value).unwrap_or(relpath),
                b"georef" => is_georeferenced = attr.as_bool().unwrap_or(false),
                b"group" => group = parse_attr(attr.value).unwrap_or(-1),
                _ => {}
            }
        }

        let mut transformations = None;
        let mut crs_spec = String::new();
        let mut track_crs_spec = String::new();
        let mut projected_crs_spec = String::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(child) => match child.local_name().as_ref() {
                    b"transformations" => {
                        transformations = Some(Transformations::parse(reader, &child)?);
                    }
                    b"crs_spec" => {
                        crs_spec = crate::notes::parse(reader)?;
                    }
                    b"projected_crs_spec" => {
                        projected_crs_spec = crate::notes::parse(reader)?;
                    }
                    b"track_crs_spec" => {
                        track_crs_spec = crate::notes::parse(reader)?;
                    }
                    _ => {}
                },
                Event::End(be) if be.local_name().as_ref() == b"template" => break,
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in template".into(),
                    ));
                }
                _ => {}
            }
        }

        let common = TemplateCommon {
            is_open,
            name,
            path,
            relpath,
            is_georeferenced,
            group,
            transformations,
        };

        let template = match template_type.as_str() {
            "TemplateTrack" => Template::Track(TrackTemplate {
                common,
                track_crs_spec,
                projected_crs_spec,
            }),
            "TemplateMap" => Template::Map(MapTemplate { common }),
            "GdalTemplate" => Template::Gdal(GdalTemplate { common, crs_spec }),
            "OgrTemplate" => Template::Ogr(OgrTemplate {
                common,
                crs_spec,
                track_crs_spec,
                projected_crs_spec,
            }),
            "TemplateImage" => Template::Image(ImageTemplate { common }),
            _ => return Err(Error::TemplateError),
        };
        Ok(template)
    }

    pub(super) fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        let common = self.get_common();

        let mut start = BytesStart::new("template").with_attributes([
            ("type", self.type_name()),
            ("open", common.is_open.to_string().as_str()),
            ("name", common.name.as_str()),
            (
                "path",
                quick_xml::escape::unescape(common.path.to_string_lossy().as_ref())?.as_ref(),
            ),
            (
                "relpath",
                quick_xml::escape::unescape(common.relpath.to_string_lossy().as_ref())?.as_ref(),
            ),
        ]);

        if common.is_georeferenced {
            start.push_attribute(("georef", "true"));
        }
        if common.group >= 0 {
            start.push_attribute(("group", common.group.to_string().as_str()));
        }

        writer.write_event(Event::Start(start))?;

        if let Some(ref t) = common.transformations {
            t.write(writer)?;
        }

        match &self {
            Template::Gdal(t) => t.write(writer)?,
            Template::Track(t) => t.write(writer)?,
            Template::Ogr(t) => t.write(writer)?,
            Template::Image(t) => t.write(writer)?,
            Template::Map(t) => t.write(writer)?,
        }

        writer.write_event(Event::End(BytesEnd::new("template")))?;
        Ok(())
    }
}

impl Transformations {
    pub(super) fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        let mut start = BytesStart::new("transformation");
        match self.adjustment {
            AdjustmentState::Adjusted => start.push_attribute(("adjusted", "true")),
            AdjustmentState::AdjustmentDirty => start.push_attribute(("adjustment_dirty", "true")),
            AdjustmentState::NoAdjustment => {}
        }
        start.push_attribute(("passpoints", self.passpoints.len().to_string().as_str()));

        writer.write_event(Event::Start(start))?;

        self.active_transform.write(writer, "active")?;
        self.other_transform.write(writer, "other")?;

        for pp in &self.passpoints {
            pp.write(writer)?;
        }

        if let Some(map_to_template) = &self.map_to_template {
            map_to_template.write(writer, "map_to_template")?;
        }
        if let Some(template_to_map) = &self.template_to_map {
            template_to_map.write(writer, "template_to_map")?;
        }
        if let Some(template_to_map_other) = &self.template_to_map_other {
            template_to_map_other.write(writer, "template_to_map_other")?;
        }

        writer.write_event(Event::End(BytesEnd::new("transformations")))?;
        Ok(())
    }
}

/// The common properties shared by all template types.
#[derive(Debug, Clone)]
pub struct TemplateCommon {
    /// Whether the template file was open (loaded) when the file was saved.
    pub is_open: bool,
    /// The filename without path, e.g. `"map.bmp"`.
    pub name: String,
    /// Absolute path to the template file.
    pub path: PathBuf,
    /// Path relative to the map file.
    pub relpath: PathBuf,
    /// Whether the template is in georeferenced mode.
    pub is_georeferenced: bool,
    /// Template group number (-1 = ungrouped).
    pub group: i32,
    /// Transformation data for non-georeferenced templates.
    pub transformations: Option<Transformations>,
}

impl TemplateCommon {
    /// Returns true if this template is georeferenced.
    pub fn is_georeferenced(&self) -> bool {
        self.is_georeferenced
    }

    /// Returns the active template transform, if present.
    pub fn active_transform(&self) -> Option<&TemplateTransform> {
        self.transformations.as_ref().map(|t| &t.active_transform)
    }

    /// Returns the other (inactive) template transform.
    pub fn other_transform(&self) -> Option<&TemplateTransform> {
        self.transformations.as_ref().map(|t| &t.other_transform)
    }

    /// Returns the pass-points, if any.
    pub fn passpoints(&self) -> &[PassPoint] {
        self.transformations.as_ref().map_or(&[], |t| &t.passpoints)
    }

    /// Returns the adjustment state.
    pub fn adjustment_state(&self) -> Option<&AdjustmentState> {
        self.transformations.as_ref().map(|t| &t.adjustment)
    }
}
