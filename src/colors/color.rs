use super::{Cmyk, CmykMode, ColorId, ColorSet, Rgb, RgbMode, SpotColorId};
use crate::{Error, NonNegativeF64, OmapSection, Result};
use crate::{
    notes,
    utils::{UnitF64, parse_attr, parse_attr_raw, try_get_attr_raw},
};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

/// A named spot color with its own CMYK/RGB representation and screen parameters.
#[derive(Debug, Clone, PartialEq)]
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
    pub fn new(
        color_name: impl Into<String>,
        spotcolor_name: impl Into<String>,
        cmyk: Cmyk,
    ) -> Self {
        Self {
            color_name: color_name.into(),
            knockout: false,
            cmyk_mode: CmykMode::Cmyk(cmyk),
            rgb_mode: RgbMode::FromCmyk,
            spotcolor_name: spotcolor_name.into(),
            screen_frequency: NonNegativeF64::default(),
            screen_angle_deg: 0.0,
        }
    }

    /// Get the effective CMYK value of this spot color.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if the color modes do not define a valid
    /// CMYK value.
    pub fn cmyk(&self) -> Result<Cmyk> {
        match self.cmyk_mode {
            CmykMode::FromSpotColors => Err(Error::ColorError),
            CmykMode::FromRgb => match self.rgb_mode {
                RgbMode::FromSpotColors | RgbMode::FromCmyk => Err(Error::ColorError),
                RgbMode::Rgb(rgb) => Ok(rgb.into()),
            },
            CmykMode::Cmyk(cmyk) => Ok(cmyk),
        }
    }

    /// Get the effective RGB value of this spot color.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if the color modes do not define a valid
    /// RGB value.
    pub fn rgb(&self) -> Result<Rgb> {
        match self.rgb_mode {
            RgbMode::FromSpotColors => Err(Error::ColorError),
            RgbMode::FromCmyk => match self.cmyk_mode {
                CmykMode::FromSpotColors | CmykMode::FromRgb => Err(Error::ColorError),
                CmykMode::Cmyk(cmyk) => Ok(cmyk.into()),
            },
            RgbMode::Rgb(rgb) => Ok(rgb),
        }
    }

    /// Set the CMYK derivation mode. Fails if a circular dependency would be created.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if `new` is not valid for a spot color or
    /// would create a circular dependency with the RGB mode.
    pub fn set_cmyk_mode(&mut self, new: CmykMode) -> Result<()> {
        if new == CmykMode::FromSpotColors
            || new == CmykMode::FromRgb && self.rgb_mode == RgbMode::FromCmyk
        {
            Err(Error::ColorError)
        } else {
            self.cmyk_mode = new;
            Ok(())
        }
    }

    /// Get the current CMYK derivation mode.
    pub fn cmyk_mode(&self) -> CmykMode {
        self.cmyk_mode
    }

    /// Set the RGB derivation mode. Fails if a circular dependency would be created.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if `new` is not valid for a spot color or
    /// would create a circular dependency with the CMYK mode.
    pub fn set_rgb_mode(&mut self, new: RgbMode) -> Result<()> {
        if new == RgbMode::FromSpotColors
            || new == RgbMode::FromCmyk && self.cmyk_mode == CmykMode::FromRgb
        {
            Err(Error::ColorError)
        } else {
            self.rgb_mode = new;
            Ok(())
        }
    }

    /// Get the current RGB derivation mode.
    pub fn rgb_mode(&self) -> RgbMode {
        self.rgb_mode
    }

    /// Get the display name of this spot color.
    pub fn name(&self) -> &str {
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
        let cmyk = self.cmyk()?;
        writer.write_event(Event::Start(BytesStart::new("color").with_attributes([
            ("priority", priority.to_string().as_str()),
            ("name", self.name()),
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorComponent {
    /// in range [0, 1]
    pub factor: UnitF64,
    /// the spot color being mixed in
    pub color: SpotColorId,
}

/// A color that is a weighted mixture of one or more spot colors.
#[derive(Debug, Clone, PartialEq)]
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
    pub fn new(color_name: impl Into<String>, components: Vec<ColorComponent>) -> Self {
        Self {
            color_name: color_name.into(),
            knockout: false,
            cmyk_mode: CmykMode::FromSpotColors,
            rgb_mode: RgbMode::FromSpotColors,
            components,
        }
    }

    /// Get the effective CMYK value of this mixed color.
    ///
    /// Takes the [`ColorSet`] that owns the component spot colors.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if a component is unavailable or the
    /// color modes form an invalid dependency.
    pub fn cmyk(&self, color_set: &ColorSet) -> Result<Cmyk> {
        match self.cmyk_mode {
            CmykMode::FromSpotColors => self.cmyk_from_spotcolors(color_set),
            CmykMode::FromRgb => match self.rgb_mode {
                RgbMode::FromSpotColors => {
                    self.rgb_from_spotcolors(color_set).map(|rgb| rgb.into())
                }
                RgbMode::FromCmyk => Err(Error::ColorError),
                RgbMode::Rgb(rgb) => Ok(rgb.into()),
            },
            CmykMode::Cmyk(cmyk) => Ok(cmyk),
        }
    }

    /// Get the effective RGB value of this mixed color.
    ///
    /// Takes the [`ColorSet`] that owns the component spot colors.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if a component is unavailable or the
    /// color modes form an invalid dependency.
    pub fn rgb(&self, color_set: &ColorSet) -> Result<Rgb> {
        match self.rgb_mode {
            RgbMode::FromSpotColors => self.rgb_from_spotcolors(color_set),
            RgbMode::FromCmyk => match self.cmyk_mode {
                CmykMode::FromSpotColors => {
                    self.cmyk_from_spotcolors(color_set).map(|cmyk| cmyk.into())
                }
                CmykMode::FromRgb => Err(Error::ColorError),
                CmykMode::Cmyk(cmyk) => Ok(cmyk.into()),
            },
            RgbMode::Rgb(rgb) => Ok(rgb),
        }
    }

    /// Get the display name of this mixed color.
    pub fn name(&self) -> &str {
        &self.color_name
    }

    /// Returns `true` if this color knocks out colors beneath it.
    pub fn is_knockout(&self) -> bool {
        self.knockout
    }

    /// Set the CMYK derivation mode. Fails if a circular dependency would be created.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if `new` would create a circular
    /// dependency with the RGB mode.
    pub fn set_cmyk_mode(&mut self, new: CmykMode) -> Result<()> {
        if new == CmykMode::FromRgb && self.rgb_mode == RgbMode::FromCmyk {
            Err(Error::ColorError)
        } else {
            self.cmyk_mode = new;
            Ok(())
        }
    }

    /// Get the current CMYK derivation mode.
    pub fn cmyk_mode(&self) -> CmykMode {
        self.cmyk_mode
    }

    /// Set the RGB derivation mode. Fails if a circular dependency would be created.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if `new` would create a circular
    /// dependency with the CMYK mode.
    pub fn set_rgb_mode(&mut self, new: RgbMode) -> Result<()> {
        if new == RgbMode::FromCmyk && self.cmyk_mode == CmykMode::FromRgb {
            Err(Error::ColorError)
        } else {
            self.rgb_mode = new;
            Ok(())
        }
    }

    /// Get the current RGB derivation mode.
    pub fn rgb_mode(&self) -> RgbMode {
        self.rgb_mode
    }

    /// Compute the CMYK value by blending the component spot colors.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if a component color is not in
    /// `color_set` or does not define an effective CMYK value.
    pub fn cmyk_from_spotcolors(&self, color_set: &ColorSet) -> Result<Cmyk> {
        let mut cmyk = Cmyk::default();

        for component in &self.components {
            let other = color_set
                .spot_color(component.color)
                .ok_or(Error::ColorError)?
                .cmyk()?;

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
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColorError`] if a component color is not in
    /// `color_set` or does not define an effective RGB value.
    pub fn rgb_from_spotcolors(&self, color_set: &ColorSet) -> Result<Rgb> {
        let mut rgb = Rgb::default();

        for component in &self.components {
            let other = color_set
                .spot_color(component.color)
                .ok_or(Error::ColorError)?
                .rgb()?;

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
        let cmyk = self.cmyk(color_set)?;

        writer.write_event(Event::Start(BytesStart::new("color").with_attributes([
            ("priority", priority.to_string().as_str()),
            ("name", self.name()),
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

        for component in &self.components {
            if let Some(priority) = color_set.priority_of(component.color.into()) {
                writer.write_event(Event::Empty(BytesStart::new("component").with_attributes(
                    [
                        ("factor", format!("{:.3}", component.factor.get()).as_str()),
                        ("spotcolor", priority.to_string().as_str()),
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

/// Either a [`SpotColor`] or a [`MixedColor`].
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    /// A spot color.
    SpotColor(SpotColor),
    /// A mixed color.
    MixedColor(MixedColor),
}

impl Color {
    /// Get the effective CMYK value of this color.
    ///
    /// Takes the [`ColorSet`] that owns any component spot colors.
    ///
    /// # Errors
    ///
    /// Returns an error if the definition cannot produce a CMYK value.
    pub fn cmyk(&self, color_set: &ColorSet) -> Result<Cmyk> {
        match self {
            Self::SpotColor(color) => color.cmyk(),
            Self::MixedColor(color) => color.cmyk(color_set),
        }
    }

    /// Get the effective RGB value of this color.
    ///
    /// Takes the [`ColorSet`] that owns any component spot colors.
    ///
    /// # Errors
    ///
    /// Returns an error if the definition cannot produce an RGB value.
    pub fn rgb(&self, color_set: &ColorSet) -> Result<Rgb> {
        match self {
            Self::SpotColor(color) => color.rgb(),
            Self::MixedColor(color) => color.rgb(color_set),
        }
    }

    /// Get the display name of this color.
    pub fn name(&self) -> &str {
        match self {
            Self::SpotColor(color) => color.name(),
            Self::MixedColor(color) => color.name(),
        }
    }

    /// Returns `true` if this color knocks out colors beneath it.
    pub fn is_knockout(&self) -> bool {
        match self {
            Self::SpotColor(color) => color.is_knockout(),
            Self::MixedColor(color) => color.is_knockout(),
        }
    }

    /// Set the CMYK derivation mode for this color.
    ///
    /// # Errors
    ///
    /// Returns an error if `new` would make the color definition invalid.
    pub fn set_cmyk_mode(&mut self, new: CmykMode) -> Result<()> {
        match self {
            Self::SpotColor(color) => color.set_cmyk_mode(new),
            Self::MixedColor(color) => color.set_cmyk_mode(new),
        }
    }

    /// Set the RGB derivation mode for this color.
    ///
    /// # Errors
    ///
    /// Returns an error if `new` would make the color definition invalid.
    pub fn set_rgb_mode(&mut self, new: RgbMode) -> Result<()> {
        match self {
            Self::SpotColor(color) => color.set_rgb_mode(new),
            Self::MixedColor(color) => color.set_rgb_mode(new),
        }
    }

    /// Get the current CMYK derivation mode.
    pub fn cmyk_mode(&self) -> CmykMode {
        match self {
            Self::SpotColor(color) => color.cmyk_mode(),
            Self::MixedColor(color) => color.cmyk_mode(),
        }
    }

    /// Get the current RGB derivation mode.
    pub fn rgb_mode(&self) -> RgbMode {
        match self {
            Self::SpotColor(color) => color.rgb_mode(),
            Self::MixedColor(color) => color.rgb_mode(),
        }
    }
}

impl From<SpotColor> for Color {
    fn from(value: SpotColor) -> Self {
        Self::SpotColor(value)
    }
}

impl From<MixedColor> for Color {
    fn from(value: MixedColor) -> Self {
        Self::MixedColor(value)
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
    #[expect(
        clippy::too_many_lines,
        reason = "color parsing follows the nested OMAP XML structure"
    )]
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
                    if let Ok(n) = parse_attr::<String>(attr, element.decoder()) {
                        name.push_str(&n);
                    }
                }
                b"c" => {
                    cmyk.c = UnitF64::clamped_from(
                        parse_attr_raw(attr.value).unwrap_or_else(|_| cmyk.c.get()),
                    );
                }
                b"m" => {
                    cmyk.m = UnitF64::clamped_from(
                        parse_attr_raw(attr.value).unwrap_or_else(|_| cmyk.m.get()),
                    );
                }
                b"y" => {
                    cmyk.y = UnitF64::clamped_from(
                        parse_attr_raw(attr.value).unwrap_or_else(|_| cmyk.y.get()),
                    );
                }
                b"k" => {
                    cmyk.k = UnitF64::clamped_from(
                        parse_attr_raw(attr.value).unwrap_or_else(|_| cmyk.k.get()),
                    );
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
                            .try_get_attribute("method")?
                            .map(|s| -> Result<Option<RgbMode>> {
                                match s.value.as_ref() {
                                    b"custom" => {
                                        let r = UnitF64::clamped_from(
                                            try_get_attr_raw(&bytes_start, "r")?.unwrap_or(0.),
                                        );
                                        let g = UnitF64::clamped_from(
                                            try_get_attr_raw(&bytes_start, "g")?.unwrap_or(0.),
                                        );
                                        let b = UnitF64::clamped_from(
                                            try_get_attr_raw(&bytes_start, "b")?.unwrap_or(0.),
                                        );
                                        Ok(Some(RgbMode::Rgb(Rgb { r, g, b })))
                                    }
                                    b"spotcolor" => Ok(Some(RgbMode::FromSpotColors)),
                                    b"cmyk" => Ok(Some(RgbMode::FromCmyk)),
                                    _ => Ok(None),
                                }
                            })
                            .transpose()?
                            .flatten()
                        {
                            rgb_mode = mode;
                        }
                    }
                    b"spotcolors" => {
                        knockout = try_get_attr_raw(&bytes_start, "knockout")
                            .ok()
                            .flatten()
                            .unwrap_or(false);

                        loop {
                            match reader.read_event_into(&mut buf)? {
                                Event::Start(bytes_start) => {
                                    // if the next event is called namedcolor we've got a new spotcolor
                                    match bytes_start.local_name().as_ref() {
                                        b"namedcolor" => {
                                            is_spotcolor = true;
                                            spot_angle =
                                                try_get_attr_raw(&bytes_start, "screen_angle")
                                                    .ok()
                                                    .flatten()
                                                    .unwrap_or(0.);
                                            spot_frequency =
                                                try_get_attr_raw(&bytes_start, "screen_frequency")
                                                    .ok()
                                                    .flatten()
                                                    .unwrap_or(0.);
                                            spotcolor_name = notes::parse(reader)?;
                                        }
                                        // if the next events are called components we have a new mixed color
                                        // we need to be carefull as the components that are refereneced may not be defined yet
                                        // so we cannot complete the color components untill all colors have been read.
                                        b"component" => {
                                            let factor = try_get_attr_raw(&bytes_start, "factor")?
                                                .unwrap_or(0.);

                                            let spotcolor_id =
                                                try_get_attr_raw(&bytes_start, "spotcolor")?
                                                    .unwrap_or(-1);
                                            spotcolor_components.push((spotcolor_id, factor));
                                        }
                                        _ => (),
                                    }
                                }
                                Event::End(bytes_end)
                                    if bytes_end.local_name().as_ref() == b"spotcolors" =>
                                {
                                    break;
                                }
                                Event::Eof => {
                                    return Err(Error::UnexpectedEof(OmapSection::Color));
                                }
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                },
                Event::End(bytes_end) if bytes_end.local_name().as_ref() == b"color" => {
                    break;
                }
                Event::Eof => return Err(Error::UnexpectedEof(OmapSection::Color)),
                _ => (),
            }
        }

        if id == usize::MAX {
            return Err(Error::MissingColorId);
        }

        if is_spotcolor {
            // fix possible bad color definition modes
            if cmyk_mode == CmykMode::FromSpotColors {
                cmyk_mode = CmykMode::Cmyk(cmyk);
            }
            if rgb_mode == RgbMode::FromSpotColors {
                rgb_mode = RgbMode::FromCmyk;
            }
            if rgb_mode == RgbMode::FromCmyk && cmyk_mode == CmykMode::FromRgb {
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
            if cmyk_mode == CmykMode::FromSpotColors && spotcolor_components.is_empty() {
                cmyk_mode = CmykMode::Cmyk(cmyk);
            }
            if rgb_mode == RgbMode::FromSpotColors && spotcolor_components.is_empty() {
                rgb_mode = RgbMode::FromCmyk;
            }
            if rgb_mode == RgbMode::FromCmyk && cmyk_mode == CmykMode::FromRgb {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolColor {
    /// A reference to a map color.
    Color(ColorId),
    /// Registration black (prints on all separations).
    RegistrationBlack,
    /// No color (transparent).
    NoColor,
}

impl SymbolColor {
    /// Create a `SymbolColor` from a file color index.
    /// -1 or missing => `NoColor`, -900 => `RegistrationBlack`, >= 0 => Color lookup.
    pub fn from_index(index: i32, color_set: &ColorSet) -> Self {
        match index {
            -900 => Self::RegistrationBlack,
            i if i >= 0 => match color_set.id_by_priority(i as usize) {
                Some(id) => Self::Color(id),
                None => Self::NoColor,
            },
            _ => Self::NoColor,
        }
    }

    /// Get the priority index of this color in the color set.
    /// Returns -1 for `NoColor`, -900 for `RegistrationBlack`.
    pub fn priority(&self, color_set: &ColorSet) -> i32 {
        match self {
            Self::Color(id) => color_set.priority_of(*id).map_or(-1, |p| p as i32),
            Self::RegistrationBlack => -900,
            Self::NoColor => -1,
        }
    }

    /// The color this refers to, if it is a map color still in `color_set`.
    pub fn id(&self) -> Option<ColorId> {
        match self {
            Self::Color(id) => Some(*id),
            Self::RegistrationBlack | Self::NoColor => None,
        }
    }
}
