use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use super::{Cmyk, CmykMode, ColorSet, Rgb, RgbMode};
use crate::{Error, NonNegativeF64, Result};
use crate::{
    notes,
    utils::{UnitF64, parse_attr, parse_attr_raw, try_get_attr_raw},
};

/// A named spot color with its own CMYK/RGB representation and screen parameters.
#[derive(Debug, Clone)]
pub struct SpotColor {
    /// The display name of this color.
    pub color_name: String,
    /// Whether this color knocks out colors beneath it.
    pub knockout: bool,
    cmyk_mode: CmykMode, // not allowed to be FromSpotColors and both this and rgb_mode cannot point at eachother
    rgb_mode: RgbMode,   // same as above
    /// The internal spot-color name used in printing.
    pub spotcolor_name: String,
    /// Screen ruling frequency (lines per inch).
    pub screen_frequency: NonNegativeF64,
    /// Screen ruling angle in degrees.
    pub screen_angle_deg: f64,
}

impl SpotColor {
    /// Create a new spot color with the given name and CMYK values.
    ///
    /// The RGB mode defaults to `FromCmyk` and screen parameters to `150.0` frequency and `0.0` angle.
    pub fn new(color_name: String, spotcolor_name: String, cmyk: Cmyk) -> Self {
        SpotColor {
            color_name,
            knockout: false,
            cmyk_mode: CmykMode::Cmyk(cmyk),
            rgb_mode: RgbMode::FromCmyk,
            spotcolor_name,
            screen_frequency: NonNegativeF64::default(),
            screen_angle_deg: 0.0,
        }
    }

    /// Get the effective CMYK value of this spot color.
    pub fn get_cmyk(&self) -> Result<Cmyk> {
        match self.cmyk_mode {
            CmykMode::FromSpotColors => Err(Error::ColorError),
            CmykMode::FromRgb => match self.rgb_mode {
                RgbMode::FromSpotColors => Err(Error::ColorError),
                RgbMode::FromCmyk => Err(Error::ColorError),
                RgbMode::Rgb(rgb) => Ok(rgb.into()),
            },
            CmykMode::Cmyk(cmyk) => Ok(cmyk),
        }
    }

    /// Get the effective RGB value of this spot color.
    pub fn get_rgb(&self) -> Result<Rgb> {
        match self.rgb_mode {
            RgbMode::FromSpotColors => Err(Error::ColorError),
            RgbMode::FromCmyk => match self.cmyk_mode {
                CmykMode::FromSpotColors => Err(Error::ColorError),
                CmykMode::FromRgb => Err(Error::ColorError),
                CmykMode::Cmyk(cmyk) => Ok(cmyk.into()),
            },
            RgbMode::Rgb(rgb) => Ok(rgb),
        }
    }

    /// Set the CMYK derivation mode. Fails if a circular dependency would be created.
    pub fn set_cmyk_mode(&mut self, new: CmykMode) -> Result<()> {
        if let CmykMode::FromSpotColors = new {
            Err(Error::ColorError)
        } else if let CmykMode::FromRgb = new
            && let RgbMode::FromCmyk = self.rgb_mode
        {
            Err(Error::ColorError)
        } else {
            self.cmyk_mode = new;
            Ok(())
        }
    }

    /// Get the current CMYK derivation mode.
    pub fn get_cmyk_mode(&self) -> CmykMode {
        self.cmyk_mode
    }

    /// Set the RGB derivation mode. Fails if a circular dependency would be created.
    pub fn set_rgb_mode(&mut self, new: RgbMode) -> Result<()> {
        if let RgbMode::FromSpotColors = new {
            Err(Error::ColorError)
        } else if let RgbMode::FromCmyk = new
            && let CmykMode::FromRgb = self.cmyk_mode
        {
            Err(Error::ColorError)
        } else {
            self.rgb_mode = new;
            Ok(())
        }
    }

    /// Get the current RGB derivation mode.
    pub fn get_rgb_mode(&self) -> RgbMode {
        self.rgb_mode
    }

    /// Get the display name of this spot color.
    pub fn get_name(&self) -> &str {
        &self.color_name
    }

    /// Returns `true` if this color knocks out colors beneath it.
    pub fn is_knockout(&self) -> bool {
        self.knockout
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        priority: usize,
    ) -> Result<()> {
        let cmyk = self.get_cmyk()?;
        writer.write_event(Event::Start(BytesStart::new("color").with_attributes([
            ("priority", priority.to_string().as_str()),
            ("name", &quick_xml::escape::escape(self.get_name())),
            ("c", format!("{:.3}", cmyk.c.get()).as_str()),
            ("m", format!("{:.3}", cmyk.m.get()).as_str()),
            ("y", format!("{:.3}", cmyk.y.get()).as_str()),
            ("k", format!("{:.3}", cmyk.k.get()).as_str()),
            ("opacity", "1"),
        ])))?;
        writer.write_event(Event::Start(
            BytesStart::new("spotcolors")
                .with_attributes([("knockout", self.knockout.to_string().as_str())]),
        ))?;
        writer.write_event(Event::Start(BytesStart::new("namedcolor").with_attributes(
            [
                (
                    "screen_angle",
                    format!("{:.1}", self.screen_angle_deg).as_str(),
                ),
                (
                    "screen_frequency",
                    format!("{:.1}", self.screen_frequency.get()).as_str(),
                ),
            ],
        )))?;
        writer.write_event(Event::Text(BytesText::new(&self.spotcolor_name)))?;
        writer.write_event(Event::End(BytesEnd::new("namedcolor")))?;
        writer.write_event(Event::End(BytesEnd::new("spotcolors")))?;
        self.cmyk_mode.write(writer)?;
        self.rgb_mode.write(writer)?;
        writer.write_event(Event::End(BytesEnd::new("color")))?;
        Ok(())
    }
}

/// A weighted reference to a spot color, used as a component in [`MixedColor`].
#[derive(Debug, Clone)]
pub struct ColorComponent {
    /// in range [0, 1]
    pub factor: UnitF64,
    /// weak reference to a spotcolor
    pub color: Weak<RefCell<SpotColor>>,
}

/// A color that is a weighted mixture of one or more spot colors.
#[derive(Debug, Clone)]
pub struct MixedColor {
    /// The display name of this mixed color.
    pub color_name: String,
    /// Whether this color knocks out colors beneath it.
    pub knockout: bool,
    cmyk_mode: CmykMode,
    rgb_mode: RgbMode,
    /// The spot-color components and their weights.
    pub components: Vec<ColorComponent>,
}

impl MixedColor {
    /// Create a new mixed color with the given name and spot-color components.
    ///
    /// Both CMYK and RGB modes default to `FromSpotColors`.
    pub fn new(color_name: String, components: Vec<ColorComponent>) -> Self {
        MixedColor {
            color_name,
            knockout: false,
            cmyk_mode: CmykMode::FromSpotColors,
            rgb_mode: RgbMode::FromSpotColors,
            components,
        }
    }

    /// Get the effective CMYK value of this mixed color.
    pub fn get_cmyk(&self) -> Result<Cmyk> {
        match self.cmyk_mode {
            CmykMode::FromSpotColors => self.cmyk_from_spotcolors(),
            CmykMode::FromRgb => match self.rgb_mode {
                RgbMode::FromSpotColors => self.rgb_from_spotcolors().map(|rgb| rgb.into()),
                RgbMode::FromCmyk => Err(Error::ColorError),
                RgbMode::Rgb(rgb) => Ok(rgb.into()),
            },
            CmykMode::Cmyk(cmyk) => Ok(cmyk),
        }
    }

    /// Get the effective RGB value of this mixed color.
    pub fn get_rgb(&self) -> Result<Rgb> {
        match self.rgb_mode {
            RgbMode::FromSpotColors => self.rgb_from_spotcolors(),
            RgbMode::FromCmyk => match self.cmyk_mode {
                CmykMode::FromSpotColors => self.cmyk_from_spotcolors().map(|cmyk| cmyk.into()),
                CmykMode::FromRgb => Err(Error::ColorError),
                CmykMode::Cmyk(cmyk) => Ok(cmyk.into()),
            },
            RgbMode::Rgb(rgb) => Ok(rgb),
        }
    }

    /// Get the display name of this mixed color.
    pub fn get_name(&self) -> &str {
        &self.color_name
    }

    /// Returns `true` if this color knocks out colors beneath it.
    pub fn is_knockout(&self) -> bool {
        self.knockout
    }

    /// Set the CMYK derivation mode. Fails if a circular dependency would be created.
    pub fn set_cmyk_mode(&mut self, new: CmykMode) -> Result<()> {
        if let CmykMode::FromRgb = new
            && let RgbMode::FromCmyk = self.rgb_mode
        {
            Err(Error::ColorError)
        } else {
            self.cmyk_mode = new;
            Ok(())
        }
    }

    /// Get the current CMYK derivation mode.
    pub fn get_cmyk_mode(&self) -> CmykMode {
        self.cmyk_mode
    }

    /// Set the RGB derivation mode. Fails if a circular dependency would be created.
    pub fn set_rgb_mode(&mut self, new: RgbMode) -> Result<()> {
        if let RgbMode::FromCmyk = new
            && let CmykMode::FromRgb = self.cmyk_mode
        {
            Err(Error::ColorError)
        } else {
            self.rgb_mode = new;
            Ok(())
        }
    }

    /// Get the current RGB derivation mode.
    pub fn get_rgb_mode(&self) -> RgbMode {
        self.rgb_mode
    }

    /// Compute the CMYK value by blending the component spot colors.
    pub fn cmyk_from_spotcolors(&self) -> Result<Cmyk> {
        let mut cmyk = Cmyk::default();

        for component in self.components.iter() {
            let other = component
                .color
                .upgrade()
                .ok_or(Error::ColorError)?
                .borrow()
                .get_cmyk()?;

            cmyk.c = UnitF64::clamped_from(
                cmyk.c.get() + component.factor.get() * other.c.get() * (1.0 - cmyk.c.get()),
            );
            cmyk.m = UnitF64::clamped_from(
                cmyk.m.get() + component.factor.get() * other.m.get() * (1.0 - cmyk.m.get()),
            );
            cmyk.y = UnitF64::clamped_from(
                cmyk.y.get() + component.factor.get() * other.y.get() * (1.0 - cmyk.y.get()),
            );
            cmyk.k = UnitF64::clamped_from(
                cmyk.k.get() + component.factor.get() * other.k.get() * (1.0 - cmyk.k.get()),
            );
        }
        Ok(cmyk)
    }

    /// Compute the RGB value by blending the component spot colors.
    pub fn rgb_from_spotcolors(&self) -> Result<Rgb> {
        let mut rgb = Rgb::default();

        for component in self.components.iter() {
            let other = component
                .color
                .upgrade()
                .ok_or(Error::ColorError)?
                .borrow()
                .get_rgb()?;

            rgb.r = UnitF64::clamped_from(
                rgb.r.get() * (1.0 - component.factor.get() * (1.0 - other.r.get())),
            );
            rgb.g = UnitF64::clamped_from(
                rgb.g.get() * (1.0 - component.factor.get() * (1.0 - other.g.get())),
            );
            rgb.b = UnitF64::clamped_from(
                rgb.b.get() * (1.0 - component.factor.get() * (1.0 - other.b.get())),
            );
        }
        Ok(rgb)
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        priority: usize,
        color_set: &ColorSet,
    ) -> Result<()> {
        let cmyk = self.get_cmyk()?;

        writer.write_event(Event::Start(BytesStart::new("color").with_attributes([
            ("priority", priority.to_string().as_str()),
            ("name", &quick_xml::escape::escape(self.get_name())),
            ("c", format!("{:.3}", cmyk.c.get()).as_str()),
            ("m", format!("{:.3}", cmyk.m.get()).as_str()),
            ("y", format!("{:.3}", cmyk.y.get()).as_str()),
            ("k", format!("{:.3}", cmyk.k.get()).as_str()),
            ("opacity", "1"),
        ])))?;
        writer.write_event(Event::Start(
            BytesStart::new("spotcolors")
                .with_attributes([("knockout", self.knockout.to_string().as_str())]),
        ))?;

        for component in self.components.iter() {
            if let Some(id) = color_set.iter().position(|color| match color.downgrade() {
                WeakColor::SpotColor(weak) => component.color.ptr_eq(&weak),
                _ => false,
            }) {
                writer.write_event(Event::Empty(BytesStart::new("component").with_attributes(
                    [
                        ("factor", format!("{:.3}", component.factor.get()).as_str()),
                        ("spotcolor", id.to_string().as_str()),
                    ],
                )))?;
            }
        }
        writer.write_event(Event::End(BytesEnd::new("spotcolors")))?;
        self.cmyk_mode.write(writer)?;
        self.rgb_mode.write(writer)?;
        writer.write_event(Event::End(BytesEnd::new("color")))?;
        Ok(())
    }
}

/// A non-owning reference to either a [`SpotColor`] or [`MixedColor`].
#[derive(Debug, Clone)]
pub enum WeakColor {
    /// A weak reference to a spot color.
    SpotColor(Weak<RefCell<SpotColor>>),
    /// A weak reference to a mixed color.
    MixedColor(Weak<RefCell<MixedColor>>),
}

impl WeakColor {
    /// Attempt to upgrade the weak reference to a strong [`Color`].
    pub fn upgrade(&self) -> Option<Color> {
        match self {
            WeakColor::SpotColor(weak) => weak.upgrade().map(Color::SpotColor),
            WeakColor::MixedColor(weak) => weak.upgrade().map(Color::MixedColor),
        }
    }
}

impl PartialEq for WeakColor {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SpotColor(l0), Self::SpotColor(r0)) => l0.ptr_eq(r0),
            (Self::MixedColor(l0), Self::MixedColor(r0)) => l0.ptr_eq(r0),
            _ => false,
        }
    }
}

/// An owning reference to either a [`SpotColor`] or [`MixedColor`].
#[derive(Debug, Clone)]
pub enum Color {
    /// A reference-counted spot color.
    SpotColor(Rc<RefCell<SpotColor>>),
    /// A reference-counted mixed color.
    MixedColor(Rc<RefCell<MixedColor>>),
}

impl Color {
    /// Get the effective CMYK value of this color.
    pub fn get_cmyk(&self) -> Result<Cmyk> {
        let cmyk = match self {
            Color::SpotColor(ref_cell) => ref_cell.try_borrow().map(|c| c.get_cmyk()),
            Color::MixedColor(ref_cell) => ref_cell.try_borrow().map(|c| c.get_cmyk()),
        }??;
        Ok(cmyk)
    }

    /// Get the effective RGB value of this color.
    pub fn get_rgb(&self) -> Result<Rgb> {
        let rgb = match self {
            Color::SpotColor(ref_cell) => ref_cell.try_borrow().map(|c| c.get_rgb()),
            Color::MixedColor(ref_cell) => ref_cell.try_borrow().map(|c| c.get_rgb()),
        }??;
        Ok(rgb)
    }

    /// Returns `true` if this color knocks out colors beneath it.
    pub fn is_knockout(&self) -> Result<bool> {
        let ko = match self {
            Color::SpotColor(ref_cell) => ref_cell.try_borrow().map(|c| c.is_knockout()),
            Color::MixedColor(ref_cell) => ref_cell.try_borrow().map(|c| c.is_knockout()),
        }?;
        Ok(ko)
    }

    /// Set the CMYK derivation mode for this color.
    pub fn set_cmyk_mode(&mut self, new: CmykMode) -> Result<()> {
        match self {
            Color::SpotColor(ref_cell) => ref_cell.try_borrow_mut()?.set_cmyk_mode(new),
            Color::MixedColor(ref_cell) => ref_cell.try_borrow_mut()?.set_cmyk_mode(new),
        }?;
        Ok(())
    }

    /// Set the RGB derivation mode for this color.
    pub fn set_rgb_mode(&mut self, new: RgbMode) -> Result<()> {
        match self {
            Color::SpotColor(ref_cell) => ref_cell.try_borrow_mut()?.set_rgb_mode(new),
            Color::MixedColor(ref_cell) => ref_cell.try_borrow_mut()?.set_rgb_mode(new),
        }?;
        Ok(())
    }

    /// Get the current CMYK derivation mode.
    pub fn get_cmyk_mode(&self) -> Result<CmykMode> {
        let mode = match self {
            Color::SpotColor(ref_cell) => ref_cell.try_borrow().map(|c| c.get_cmyk_mode()),
            Color::MixedColor(ref_cell) => ref_cell.try_borrow().map(|c| c.get_cmyk_mode()),
        }?;
        Ok(mode)
    }

    /// Get the current RGB derivation mode.
    pub fn get_rgb_mode(&self) -> Result<RgbMode> {
        let mode = match self {
            Color::SpotColor(ref_cell) => ref_cell.try_borrow().map(|c| c.get_rgb_mode()),
            Color::MixedColor(ref_cell) => ref_cell.try_borrow().map(|c| c.get_rgb_mode()),
        }?;
        Ok(mode)
    }

    /// Create a non-owning [`WeakColor`] from this color.
    pub fn downgrade(&self) -> WeakColor {
        match self {
            Color::SpotColor(ref_cell) => WeakColor::SpotColor(Rc::downgrade(ref_cell)),
            Color::MixedColor(ref_cell) => WeakColor::MixedColor(Rc::downgrade(ref_cell)),
        }
    }
}

pub(super) enum ColorParseReturn {
    Spot {
        color: SpotColor,
        priority: usize,
    },
    Mix {
        color: MixedColor,
        priority: usize,
        components: Vec<(i32, f64)>,
    },
}

impl Color {
    /// Parsing return both the parsed color (or almost parsed color) and the spotcolor references which might not be parseable yet and must wait
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        element: &BytesStart<'_>,
    ) -> Result<ColorParseReturn> {
        let mut name = String::new();
        let mut cmyk = Cmyk::default();
        let mut id = usize::MAX;

        for attr in element.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"name" => {
                    if let Some(n) = parse_attr::<String>(attr, element.decoder()) {
                        name.push_str(&n);
                    }
                }
                b"c" => {
                    cmyk.c =
                        UnitF64::clamped_from(parse_attr_raw(attr.value).unwrap_or(cmyk.c.get()))
                }
                b"m" => {
                    cmyk.m =
                        UnitF64::clamped_from(parse_attr_raw(attr.value).unwrap_or(cmyk.m.get()))
                }
                b"y" => {
                    cmyk.y =
                        UnitF64::clamped_from(parse_attr_raw(attr.value).unwrap_or(cmyk.y.get()))
                }
                b"k" => {
                    cmyk.k =
                        UnitF64::clamped_from(parse_attr_raw(attr.value).unwrap_or(cmyk.k.get()))
                }
                b"priority" => id = parse_attr_raw(attr.value).unwrap_or(id),
                _ => (),
            }
        }

        let mut is_spotcolor = false;
        let mut cmyk_mode = CmykMode::Cmyk(cmyk);
        let mut rgb_mode = RgbMode::FromCmyk;

        let mut spot_angle = 0.;
        let mut spot_frequency = 0.;
        let mut spotcolor_name = String::new();
        let mut spotcolor_components = Vec::new();
        let mut knockout = false;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(bytes_start) => match bytes_start.local_name().as_ref() {
                    b"cmyk" => {
                        if let Some(mode) = bytes_start
                            .try_get_attribute("method")
                            .ok()
                            .flatten()
                            .and_then(|s| match s.value.as_ref() {
                                b"custom" => Some(CmykMode::Cmyk(cmyk)),
                                b"spotcolor" => Some(CmykMode::FromSpotColors),
                                b"rgb" => Some(CmykMode::FromRgb),
                                _ => None,
                            })
                        {
                            cmyk_mode = mode;
                        }
                    }
                    b"rgb" => {
                        if let Some(mode) = bytes_start
                            .try_get_attribute("method")
                            .ok()
                            .flatten()
                            .and_then(|s| match s.value.as_ref() {
                                b"custom" => {
                                    let r = UnitF64::clamped_from(
                                        try_get_attr_raw(&bytes_start, "r").unwrap_or(0.),
                                    );
                                    let g = UnitF64::clamped_from(
                                        try_get_attr_raw(&bytes_start, "g").unwrap_or(0.),
                                    );
                                    let b = UnitF64::clamped_from(
                                        try_get_attr_raw(&bytes_start, "b").unwrap_or(0.),
                                    );
                                    Some(RgbMode::Rgb(Rgb { r, g, b }))
                                }
                                b"spotcolor" => Some(RgbMode::FromSpotColors),
                                b"cmyk" => Some(RgbMode::FromCmyk),
                                _ => None,
                            })
                        {
                            rgb_mode = mode;
                        }
                    }
                    b"spotcolors" => {
                        knockout = try_get_attr_raw(&bytes_start, "knockout").unwrap_or(false);

                        loop {
                            match reader.read_event_into(&mut buf)? {
                                Event::Start(bytes_start) => {
                                    // if the next event is called namedcolor we've got a new spotcolor
                                    match bytes_start.local_name().as_ref() {
                                        b"namedcolor" => {
                                            is_spotcolor = true;
                                            spot_angle =
                                                try_get_attr_raw(&bytes_start, "screen_angle")
                                                    .unwrap_or(0.);
                                            spot_frequency =
                                                try_get_attr_raw(&bytes_start, "screen_frequency")
                                                    .unwrap_or(0.);
                                            spotcolor_name = notes::parse(reader)?;
                                        }
                                        // if the next events are called components we have a new mixed color
                                        // we need to be carefull as the components that are refereneced may not be defined yet
                                        // so we cannot complete the color components untill all colors have been read.
                                        b"component" => {
                                            let factor = try_get_attr_raw(&bytes_start, "factor")
                                                .unwrap_or(0.);

                                            let spotcolor_id =
                                                try_get_attr_raw(&bytes_start, "spotcolor")
                                                    .unwrap_or(-1);
                                            spotcolor_components.push((spotcolor_id, factor));
                                        }
                                        _ => (),
                                    }
                                }
                                Event::End(bytes_end) => {
                                    if bytes_end.local_name().as_ref() == b"spotcolors" {
                                        break;
                                    }
                                }
                                Event::Eof => {
                                    return Err(Error::ParseOmapFileError("Early EOF".to_string()));
                                }
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                },
                Event::End(bytes_end) => {
                    if bytes_end.local_name().as_ref() == b"color" {
                        break;
                    }
                }
                Event::Eof => return Err(Error::ParseOmapFileError("Early EOF".to_string())),
                _ => (),
            }
        }

        if id == usize::MAX {
            return Err(Error::ParseOmapFileError(
                "Could not parse color".to_string(),
            ));
        }

        if is_spotcolor {
            // fix possible bad color definition modes
            if let CmykMode::FromSpotColors = cmyk_mode {
                cmyk_mode = CmykMode::Cmyk(cmyk);
            }
            if let RgbMode::FromSpotColors = rgb_mode {
                rgb_mode = RgbMode::FromCmyk;
            }
            if let RgbMode::FromCmyk = rgb_mode
                && let CmykMode::FromRgb = cmyk_mode
            {
                cmyk_mode = CmykMode::Cmyk(cmyk);
            }

            Ok(ColorParseReturn::Spot {
                color: SpotColor {
                    color_name: name,
                    knockout,
                    cmyk_mode,
                    rgb_mode,
                    spotcolor_name,
                    screen_frequency: NonNegativeF64::clamped_from(spot_frequency),
                    screen_angle_deg: spot_angle,
                },
                priority: id,
            })
        } else {
            // fix possible bad color definition modes
            if let CmykMode::FromSpotColors = cmyk_mode
                && spotcolor_components.is_empty()
            {
                cmyk_mode = CmykMode::Cmyk(cmyk);
            }
            if let RgbMode::FromSpotColors = rgb_mode
                && spotcolor_components.is_empty()
            {
                rgb_mode = RgbMode::FromCmyk;
            }
            if let RgbMode::FromCmyk = rgb_mode
                && let CmykMode::FromRgb = cmyk_mode
            {
                cmyk_mode = CmykMode::Cmyk(cmyk);
            }
            Ok(ColorParseReturn::Mix {
                color: MixedColor {
                    color_name: name,
                    knockout,
                    cmyk_mode,
                    rgb_mode,
                    components: Vec::new(),
                },
                priority: id,
                components: spotcolor_components,
            })
        }
    }
}

/// A color reference used by symbols: a regular color, registration black, or no color.
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolColor {
    /// A reference to a map color.
    Color(WeakColor),
    /// Registration black (prints on all separations).
    RegistrationBlack,
    /// No color (transparent).
    NoColor,
}

impl SymbolColor {
    /// Create a SymbolColor from a file color index.
    /// -1 or missing => NoColor, -900 => RegistrationBlack, >= 0 => Color lookup.
    pub fn from_index(index: i32, color_set: &ColorSet) -> Self {
        match index {
            -900 => SymbolColor::RegistrationBlack,
            i if i >= 0 => match color_set.get_weak_color_by_priority(i as usize) {
                Some(weak) => SymbolColor::Color(weak),
                None => SymbolColor::NoColor,
            },
            _ => SymbolColor::NoColor,
        }
    }

    /// Get the priority index of this color in the color set.
    /// Returns -1 for `NoColor`, -900 for `RegistrationBlack`.
    pub fn get_priority(&self, color_set: &ColorSet) -> i32 {
        match self {
            SymbolColor::Color(weak_color) => weak_color
                .upgrade()
                .and_then(|c| color_set.get_id_of_color(&c).map(|u| u as i32))
                .unwrap_or(-1),
            SymbolColor::RegistrationBlack => -900,
            SymbolColor::NoColor => -1,
        }
    }
}
