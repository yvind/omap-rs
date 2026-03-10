use std::str::FromStr;

use geo_types::Coord;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::{Reader, Writer};

use crate::colors::Rgb;
use crate::templates::Templates;
use crate::utils::{UnitF64, parse_attr, try_get_attr};
use crate::{Error, Result};

/// Visibility settings for a template or the map layer.
#[derive(Debug, Clone, Copy)]
pub struct TemplateVisibility {
    /// Opacity from 0.0 (invisible) to 1.0 (opaque).
    pub opacity: f64,
    /// Whether this layer is visible.
    pub visible: bool,
}

impl Default for TemplateVisibility {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            visible: false,
        }
    }
}

impl TemplateVisibility {
    fn parse_map_attrs(bs: &BytesStart<'_>) -> Self {
        let mut tv = Self::default();
        for attr in bs.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"opacity" => tv.opacity = parse_attr(attr.value).unwrap_or(tv.opacity),
                b"visible" => tv.visible = attr.as_bool().unwrap_or(tv.visible),
                _ => (),
            }
        }
        tv
    }
}

/// How the grid is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridDisplay {
    /// Grid is hidden.
    #[default]
    Hidden = 0,
    /// All grid lines are shown.
    AllLines = 1,
    /// Only horizontal lines are shown.
    HorizontalLines = 2,
    /// Only vertical lines are shown.
    VerticalLines = 3,
}

impl FromStr for GridDisplay {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "0" => Ok(GridDisplay::Hidden),
            "1" => Ok(GridDisplay::AllLines),
            "2" => Ok(GridDisplay::HorizontalLines),
            "3" => Ok(GridDisplay::VerticalLines),
            _ => Err(Error::ViewError),
        }
    }
}

impl From<u8> for GridDisplay {
    fn from(value: u8) -> GridDisplay {
        match value {
            1 => GridDisplay::AllLines,
            2 => GridDisplay::HorizontalLines,
            3 => GridDisplay::VerticalLines,
            _ => GridDisplay::Hidden,
        }
    }
}

impl AsRef<str> for GridDisplay {
    fn as_ref(&self) -> &str {
        match self {
            GridDisplay::Hidden => "0",
            GridDisplay::AllLines => "1",
            GridDisplay::HorizontalLines => "2",
            GridDisplay::VerticalLines => "3",
        }
    }
}

/// Grid alignment reference direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridAlignment {
    /// Aligned to magnetic north.
    #[default]
    MagneticNorth = 0,
    /// Aligned to grid north.
    GridNorth = 1,
    /// Aligned to true north.
    TrueNorth = 2,
}

impl FromStr for GridAlignment {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "0" => Ok(GridAlignment::MagneticNorth),
            "1" => Ok(GridAlignment::GridNorth),
            "2" => Ok(GridAlignment::TrueNorth),
            _ => Err(Error::ViewError),
        }
    }
}

impl From<u8> for GridAlignment {
    fn from(value: u8) -> GridAlignment {
        match value {
            1 => GridAlignment::GridNorth,
            2 => GridAlignment::TrueNorth,
            _ => GridAlignment::MagneticNorth,
        }
    }
}

impl AsRef<str> for GridAlignment {
    fn as_ref(&self) -> &str {
        match self {
            Self::MagneticNorth => "0",
            Self::GridNorth => "1",
            Self::TrueNorth => "2",
        }
    }
}

/// Grid spacing unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridUnit {
    /// Meters on the ground.
    #[default]
    MetersOnGround = 0,
    /// Millimetres on the map.
    MillimetresOnMap = 1,
    /// Pixels on screen.
    PixelsOnScreen = 2,
}

impl FromStr for GridUnit {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "0" => Ok(GridUnit::MetersOnGround),
            "1" => Ok(GridUnit::MillimetresOnMap),
            "2" => Ok(GridUnit::PixelsOnScreen),
            _ => Err(Error::ViewError),
        }
    }
}

impl From<u8> for GridUnit {
    fn from(value: u8) -> GridUnit {
        match value {
            1 => GridUnit::MillimetresOnMap,
            2 => GridUnit::PixelsOnScreen,
            _ => GridUnit::MetersOnGround,
        }
    }
}

impl AsRef<str> for GridUnit {
    fn as_ref(&self) -> &str {
        match self {
            GridUnit::MetersOnGround => "0",
            GridUnit::MillimetresOnMap => "1",
            GridUnit::PixelsOnScreen => "2",
        }
    }
}

/// The map grid display settings.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Rgb Grid colour parsed from a hex string, e.g. `"#646464"`.
    pub color: Rgb,
    /// Display mode.
    pub display: GridDisplay,
    /// Grid alignment reference direction.
    pub alignment: GridAlignment,
    /// Additional rotation in radians.
    pub additional_rotation: f64,
    /// Grid spacing unit.
    pub unit: GridUnit,
    /// Horizontal spacing.
    pub h_spacing: f64,
    /// Vertical spacing.
    pub v_spacing: f64,
    /// Horizontal offset.
    pub h_offset: f64,
    /// Vertical offset.
    pub v_offset: f64,
    /// Whether snapping to the grid is enabled.
    pub snapping_enabled: bool,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            color: Rgb {
                r: UnitF64::clamped_from(100. / 255.),
                g: UnitF64::clamped_from(100. / 255.),
                b: UnitF64::clamped_from(100. / 255.),
            },
            display: Default::default(),
            alignment: Default::default(),
            additional_rotation: 0.0,
            unit: Default::default(),
            h_spacing: 500.0,
            v_spacing: 500.0,
            h_offset: 0.0,
            v_offset: 0.0,
            snapping_enabled: false,
        }
    }
}

impl Grid {
    fn parse_attrs(bs: &BytesStart<'_>) -> Self {
        let mut g = Self::default();
        for attr in bs.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"color" => g.color = parse_attr(attr.value).unwrap_or(g.color),
                b"display" => g.display = parse_attr(attr.value).unwrap_or(g.display),
                b"alignment" => g.alignment = parse_attr(attr.value).unwrap_or(g.alignment),
                b"unit" => g.unit = parse_attr(attr.value).unwrap_or(g.unit),
                b"additional_rotation" => {
                    g.additional_rotation = parse_attr(attr.value).unwrap_or(g.additional_rotation)
                }
                b"h_spacing" => g.h_spacing = parse_attr(attr.value).unwrap_or(g.h_spacing),
                b"v_spacing" => g.v_spacing = parse_attr(attr.value).unwrap_or(g.v_spacing),
                b"h_offset" => g.h_offset = parse_attr(attr.value).unwrap_or(g.h_offset),
                b"v_offset" => g.v_offset = parse_attr(attr.value).unwrap_or(g.v_offset),
                b"snapping_enabled" => {
                    g.snapping_enabled = attr.as_bool().unwrap_or(g.snapping_enabled)
                }
                _ => (),
            }
        }
        g
    }

    fn write<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Empty(BytesStart::new("grid").with_attributes([
            ("color", self.color.to_hexstring().as_str()),
            ("display", self.display.as_ref()),
            ("alignment", self.alignment.as_ref()),
            (
                "additional_rotation",
                self.additional_rotation.to_string().as_str(),
            ),
            ("unit", self.unit.as_ref()),
            ("h_spacing", self.h_spacing.to_string().as_str()),
            ("v_spacing", self.v_spacing.to_string().as_str()),
            ("h_offset", self.h_offset.to_string().as_str()),
            ("v_offset", self.v_offset.to_string().as_str()),
            (
                "snapping_enabled",
                self.snapping_enabled.to_string().as_str(),
            ),
        ])))?;
        Ok(())
    }
}
/// The view onto the map, including zoom, position, rotation, grid settings,
/// and visibility of the map layer and templates.
#[derive(Debug, Clone)]
pub struct View {
    /// Grid display settings.
    pub grid: Grid,
    /// Zoom factor.
    pub zoom: f64,
    /// View rotation in radians (counter-clockwise).
    pub rotation: f64,
    /// Horizontal position of the view centre (in µm map coordinates).
    pub view_centre: Coord<i32>,
    /// Visibility of the map drawing itself.
    pub map_visibility: TemplateVisibility,
    /// Whether all templates are hidden in this view.
    pub all_templates_hidden: bool,
    /// Whether the grid is visible.
    pub grid_visible: bool,
    /// Whether overprinting simulation is enabled.
    pub overprinting_simulation_enabled: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            grid: Grid::default(),
            zoom: 1.0,
            rotation: 0.0,
            view_centre: Coord::zero(),
            map_visibility: TemplateVisibility {
                opacity: 1.0,
                visible: true,
            },
            all_templates_hidden: false,
            grid_visible: false,
            overprinting_simulation_enabled: false,
        }
    }
}

impl View {
    pub(crate) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        _event: &BytesStart<'_>,
        templates: &mut Templates,
    ) -> Result<Self> {
        let mut view = Self::default();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bs) => match bs.local_name().as_ref() {
                    b"grid" => view.grid = Grid::parse_attrs(&bs),
                    b"map_view" => view.parse_map_view(reader, &bs, templates)?,
                    _ => {}
                },
                Event::End(be) if be.local_name().as_ref() == b"view" => break,
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(view)
    }

    fn parse_map_view<R: std::io::BufRead>(
        &mut self,
        reader: &mut Reader<R>,
        bs: &BytesStart<'_>,
        templates: &mut Templates,
    ) -> Result<()> {
        self.zoom = try_get_attr(bs, "zoom").unwrap_or(1.0);
        self.rotation = try_get_attr(bs, "rotation").unwrap_or(0.0);
        self.view_centre.x = try_get_attr(bs, "position_x").unwrap_or(0);
        self.view_centre.y = try_get_attr(bs, "position_y").unwrap_or(0);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bs) => match bs.local_name().as_ref() {
                    b"map" => {
                        self.map_visibility = TemplateVisibility::parse_map_attrs(&bs);
                    }
                    b"templates" => {
                        if !templates.is_empty() {
                            self.parse_template_visibilities(reader, templates)?;
                        }
                    }
                    _ => {}
                },
                Event::End(be) if be.local_name().as_ref() == b"map_view" => break,
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(())
    }

    fn parse_template_visibilities<R: std::io::BufRead>(
        &mut self,
        reader: &mut Reader<R>,
        templates: &mut Templates,
    ) -> Result<()> {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bs) if bs.local_name().as_ref() == b"ref" => {
                    if let Some(index) = try_get_attr::<usize>(&bs, "index")
                        && index < templates.len()
                    {
                        templates.template_entries[index].visibilty =
                            TemplateVisibility::parse_map_attrs(&bs);
                    }
                }
                Event::End(be) if be.local_name().as_ref() == b"templates" => break,
                Event::Eof => break,
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn write<W: std::io::Write>(
        self,
        writer: &mut Writer<W>,
        visibilities: Vec<TemplateVisibility>,
    ) -> Result<()> {
        // <view>
        writer.write_event(Event::Start(BytesStart::new("view")))?;

        // <grid ... />
        self.grid.write(writer)?;

        let mut mv = BytesStart::new("map_view").with_attributes([
            ("zoom", format!("{:.4}", self.zoom).as_str()),
            ("position_x", self.view_centre.x.to_string().as_str()),
            ("position_y", self.view_centre.y.to_string().as_str()),
        ]);

        if self.rotation != 0.0 {
            mv.push_attribute(("rotation", format!("{:.4}", self.rotation).as_str()));
        }

        writer.write_event(Event::Start(mv))?;

        writer.write_event(Event::Empty(BytesStart::new("map").with_attributes([
            (
                "opacity",
                format!("{:.2}", self.map_visibility.opacity).as_str(),
            ),
            ("visible", self.map_visibility.visible.to_string().as_str()),
        ])))?;

        if visibilities.is_empty() {
            writer.write_event(Event::Empty(
                BytesStart::new("templates").with_attributes([("count", "0")]),
            ))?;
        } else {
            writer
                .write_event(Event::Start(BytesStart::new("templates").with_attributes(
                    [("count", visibilities.len().to_string().as_str())],
                )))?;

            for (index, vis) in visibilities.into_iter().enumerate() {
                writer.write_event(Event::Empty(BytesStart::new("ref").with_attributes([
                    ("template", index.to_string().as_str()),
                    ("opacity", format!("{:.2}", vis.opacity).as_str()),
                    ("visible", vis.visible.to_string().as_str()),
                ])))?;
            }
            writer.write_event(Event::End(BytesEnd::new("templates")))?;
        }

        // </map_view>
        writer.write_event(Event::End(BytesEnd::new("map_view")))?;

        // </view>
        writer.write_event(Event::End(BytesEnd::new("view")))?;

        Ok(())
    }
}
