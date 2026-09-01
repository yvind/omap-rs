use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::SymbolCommon;
use crate::{
    Code, Error, NonNegativeF64, OmapSection, Result,
    colors::{ColorId, ColorSet, SymbolColor},
    notes,
    utils::{self, try_get_attr_raw},
};

/// The framing mode for a text symbol.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FramingMode {
    /// No framing.
    #[default]
    NoFraming,
    /// An outline framing around each character.
    LineFraming(LineFraming),
    /// A shadow behind the text.
    ShadowFraming(ShadowFraming),
}

impl FramingMode {
    /// Get the numeric identifier for this framing mode.
    pub fn id(&self) -> u8 {
        match self {
            Self::NoFraming => 0,
            Self::LineFraming(_) => 1,
            Self::ShadowFraming(_) => 2,
        }
    }
}

/// Line-based framing (halo) around text characters.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineFraming {
    /// Color of the framing line.
    pub color: SymbolColor,
    /// Half-width of the framing line.
    pub framing_line_half_width: NonNegativeF64,
}

/// Shadow framing behind text characters.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShadowFraming {
    /// Color of the shadow.
    pub color: SymbolColor,
    /// Offset of the shadow from the text.
    pub shadow_offset: Coord<f64>,
}

/// A line drawn below the text (underline).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineBelow {
    /// Color of the line.
    pub color: SymbolColor,
    /// Width of the line.
    pub width: NonNegativeF64,
    /// Distance between the text and the line.
    pub distance: NonNegativeF64,
}

/// A text symbol definition.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextSymbol {
    /// The common symbol fields
    pub common: SymbolCommon,
    /// f.ex Arial
    pub font_family: String,
    /// Should not be more than 3 chars long
    pub icon_text: String,
    /// Color of the text
    pub color: SymbolColor,

    /// OCD custom tab positions in mm
    pub custom_tabs: Vec<NonNegativeF64>,
    /// OCD underlining
    pub line_below: Option<LineBelow>,

    /// as factor of original line spacing
    pub line_spacing: NonNegativeF64,
    /// as a factor of the space character width
    pub character_spacing: f64,
    /// this defines the font size in mm. How big the letters really are depends on the design of the font though
    pub font_size: NonNegativeF64,
    /// Spacing between paragraphs in mm.
    pub paragraph_spacing: f64,
    /// The framing mode (outline, shadow, or none).
    pub framing_mode: Option<FramingMode>,

    /// is the text allowed to be rotated
    pub is_rotatable: bool,
    /// bold text
    pub bold: bool,
    /// italix text
    pub italic: bool,
    /// underlined text
    pub underline: bool,
    /// kerning (adaptive character spacing for better readability)
    pub kerning: bool,
}

impl TextSymbol {
    /// Create a new text symbol with the given code, name, and font family.
    pub fn new(code: Code, name: impl Into<String>) -> Self {
        let common = SymbolCommon {
            code,
            name: name.into(),
            ..Default::default()
        };
        Self {
            common,
            font_family: String::from("Arial"),
            icon_text: String::new(),
            color: SymbolColor::NoColor,
            custom_tabs: Vec::new(),
            line_below: None,
            line_spacing: NonNegativeF64::one(),
            character_spacing: 0.0,
            font_size: NonNegativeF64::clamped_from(4.0),
            paragraph_spacing: 0.0,
            framing_mode: None,
            is_rotatable: false,
            bold: false,
            italic: false,
            underline: false,
            kerning: true,
        }
    }

    /// Get the display name of this text symbol.
    pub fn name(&self) -> &str {
        &self.common.name
    }

    /// Set the font family (builder-style).
    pub fn with_font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = family.into();
        self
    }

    /// Set the font size in mm (builder-style).
    pub fn with_font_size(mut self, size: NonNegativeF64) -> Self {
        self.font_size = size;
        self
    }

    /// Set the text colour (builder-style).
    pub fn with_color(mut self, color: SymbolColor) -> Self {
        self.color = color;
        self
    }

    /// Set bold style (builder-style).
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Set italic style (builder-style).
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Set whether the symbol is rotatable (builder-style).
    pub fn with_rotatable(mut self, rotatable: bool) -> Self {
        self.is_rotatable = rotatable;
        self
    }

    /// Set line spacing as factor of original (builder-style).
    pub fn with_line_spacing(mut self, spacing: NonNegativeF64) -> Self {
        self.line_spacing = spacing;
        self
    }

    /// Mark as a helper symbol (builder-style).
    pub fn as_helper_symbol(mut self) -> Self {
        self.common.is_helper_symbol = true;
        self
    }

    pub fn colors(&self) -> Vec<ColorId> {
        let mut colors = Vec::new();

        if let SymbolColor::Color(id) = &self.color {
            colors.push(*id);
        }

        if let Some(underline) = &self.line_below
            && let SymbolColor::Color(id) = &underline.color
        {
            colors.push(*id);
        }

        if let Some(framing) = &self.framing_mode {
            match framing {
                FramingMode::NoFraming => (),
                FramingMode::LineFraming(line_framing) => {
                    if let SymbolColor::Color(id) = &line_framing.color {
                        colors.push(*id);
                    }
                }
                FramingMode::ShadowFraming(shadow_framing) => {
                    if let SymbolColor::Color(id) = &shadow_framing.color {
                        colors.push(*id);
                    }
                }
            }
        }

        colors
    }

    #[expect(
        clippy::too_many_lines,
        reason = "text-symbol parsing maps a large file-format record"
    )]
    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<Self> {
        let mut common = attributes;
        let mut icon_text = String::new();
        let mut is_rotatable = false;
        let mut font_family = String::from("Arial");
        let mut font_size = NonNegativeF64::clamped_from(4.0);
        let mut bold = false;
        let mut italic = false;
        let mut underline = false;
        let mut color = SymbolColor::NoColor;
        let mut line_spacing = NonNegativeF64::one();
        let mut paragraph_spacing = 0.0;
        let mut character_spacing = 0.0;
        let mut kerning = true;
        let mut framing_mode = None;
        let mut line_below = None;
        let mut custom_tabs = Vec::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) => match e.local_name().as_ref() {
                    "description" => common.description = notes::parse(reader)?,
                    "text_symbol" => {
                        icon_text = try_get_attr_raw(&e, "icon_text")?.unwrap_or_default();
                        is_rotatable = try_get_attr_raw(&e, "rotatable")?.unwrap_or(false);
                    }
                    "font" => {
                        font_family = try_get_attr_raw(&e, "family")?
                            .unwrap_or_else(|| String::from("Arial"));
                        let fs = try_get_attr_raw(&e, "size")?.unwrap_or(4000);
                        font_size = NonNegativeF64::from_file_value(fs);
                        bold = try_get_attr_raw(&e, "bold")?.unwrap_or(false);
                        italic = try_get_attr_raw(&e, "italic")?.unwrap_or(false);
                        underline = try_get_attr_raw(&e, "underline")?.unwrap_or(false);
                    }
                    "text" => {
                        let ci = try_get_attr_raw(&e, "color")?.unwrap_or(-1);
                        color = SymbolColor::from_index(ci, color_set);
                        let ls = try_get_attr_raw(&e, "line_spacing")?.unwrap_or(1.0);
                        line_spacing = NonNegativeF64::clamped_from(ls);
                        paragraph_spacing = NonNegativeF64::from_file_value(
                            try_get_attr_raw(&e, "paragraph_spacing")?.unwrap_or(0),
                        )
                        .get();
                        character_spacing =
                            try_get_attr_raw(&e, "character_spacing")?.unwrap_or(0.0);
                        kerning = try_get_attr_raw(&e, "kerning")?.unwrap_or(false);
                    }
                    "framing" => {
                        let fc = try_get_attr_raw(&e, "color")?.unwrap_or(-1);
                        let framing_color = SymbolColor::from_index(fc, color_set);
                        let mode = try_get_attr_raw(&e, "mode")?.unwrap_or(0);
                        framing_mode = Some(match mode {
                            1 => {
                                let half_width = NonNegativeF64::from_file_value(
                                    try_get_attr_raw(&e, "line_half_width")?.unwrap_or(0),
                                );
                                FramingMode::LineFraming(LineFraming {
                                    color: framing_color,
                                    framing_line_half_width: half_width,
                                })
                            }
                            2 => {
                                let sx = try_get_attr_raw(&e, "shadow_x_offset")?.unwrap_or(0);
                                let sy = try_get_attr_raw(&e, "shadow_y_offset")?.unwrap_or(0);
                                FramingMode::ShadowFraming(ShadowFraming {
                                    color: framing_color,
                                    shadow_offset: Coord {
                                        x: utils::from_file_value(sx),
                                        y: utils::from_file_value(sy),
                                    },
                                })
                            }
                            _ => FramingMode::NoFraming,
                        });
                    }
                    "line_below" => {
                        let lc = try_get_attr_raw(&e, "color")?.unwrap_or(-1);
                        let lb_color = SymbolColor::from_index(lc, color_set);
                        let w = try_get_attr_raw(&e, "width")?.unwrap_or(0);
                        let d = try_get_attr_raw(&e, "distance")?.unwrap_or(0);
                        line_below = Some(LineBelow {
                            color: lb_color,
                            width: NonNegativeF64::from_file_value(w),
                            distance: NonNegativeF64::from_file_value(d),
                        });
                    }
                    "icon" => common.custom_icon = try_get_attr_raw(&e, "src")?,
                    "tabs" => {}
                    "tab" => {}
                    _ => {}
                },
                Event::Text(text) => {
                    if let Ok(v) = text.as_ref().parse() {
                        custom_tabs.push(NonNegativeF64::from_file_value(v));
                    }
                }
                Event::End(e) if e.local_name().as_ref() == "symbol" => {
                    break;
                }
                Event::Eof => {
                    return Err(Error::UnexpectedEof(OmapSection::TextSymbol));
                }
                _ => {}
            }
        }

        Ok(Self {
            common,
            font_family,
            icon_text,
            color,
            custom_tabs,
            line_below,
            line_spacing,
            character_spacing,
            font_size,
            paragraph_spacing,
            framing_mode,
            is_rotatable,
            bold,
            italic,
            underline,
            kerning,
        })
    }

    /// Write the type-specific body, between the halves of the shared
    /// `<symbol>` frame written by [`Symbol::write`].
    pub(super) fn write_body<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
    ) -> Result<()> {
        writer.write_event(Event::Start(
            BytesStart::new("text_symbol").with_attributes([
                ("icon_text", self.icon_text.as_str()),
                ("rotatable", self.is_rotatable.to_string().as_str()),
            ]),
        ))?;

        let mut font = BytesStart::new("font").with_attributes([
            ("family", self.font_family.as_str()),
            ("size", self.font_size.to_file_value()?.to_string().as_str()),
        ]);
        if self.bold {
            font.push_attribute(("bold", "true"));
        }
        if self.italic {
            font.push_attribute(("italic", "true"));
        }
        if self.underline {
            font.push_attribute(("underline", "true"));
        }
        writer.write_event(Event::Empty(font))?;

        let ps_file = utils::to_file_value(self.paragraph_spacing)?;
        let mut text = BytesStart::new("text").with_attributes([
            ("color", self.color.priority(color_set).to_string().as_str()),
            ("line_spacing", self.line_spacing.get().to_string().as_str()),
            ("paragraph_spacing", ps_file.to_string().as_str()),
            (
                "character_spacing",
                self.character_spacing.to_string().as_str(),
            ),
        ]);
        if self.kerning {
            text.push_attribute(("kerning", "true"));
        }
        writer.write_event(Event::Empty(text))?;

        if let Some(fm) = &self.framing_mode {
            match fm {
                FramingMode::NoFraming => {}
                FramingMode::LineFraming(lf) => {
                    writer.write_event(Event::Empty(
                        BytesStart::new("framing").with_attributes([
                            ("color", lf.color.priority(color_set).to_string().as_str()),
                            ("mode", "1"),
                            (
                                "line_half_width",
                                lf.framing_line_half_width
                                    .to_file_value()?
                                    .to_string()
                                    .as_str(),
                            ),
                        ]),
                    ))?;
                }
                FramingMode::ShadowFraming(sf) => {
                    let shadow = utils::to_file_coords(sf.shadow_offset)?;
                    writer.write_event(Event::Empty(
                        BytesStart::new("framing").with_attributes([
                            ("color", sf.color.priority(color_set).to_string().as_str()),
                            ("mode", "2"),
                            ("shadow_x_offset", shadow.x.to_string().as_str()),
                            ("shadow_y_offset", shadow.y.to_string().as_str()),
                        ]),
                    ))?;
                }
            }
        }

        if let Some(lb) = &self.line_below {
            writer.write_event(Event::Empty(BytesStart::new("line_below").with_attributes(
                [
                    ("color", lb.color.priority(color_set).to_string().as_str()),
                    ("width", lb.width.to_file_value()?.to_string().as_str()),
                    (
                        "distance",
                        lb.distance.to_file_value()?.to_string().as_str(),
                    ),
                ],
            )))?;
        }

        if !self.custom_tabs.is_empty() {
            writer.write_event(Event::Start(
                BytesStart::new("tabs")
                    .with_attributes([("count", self.custom_tabs.len().to_string().as_str())]),
            ))?;
            for tab in &self.custom_tabs {
                writer.write_event(Event::Start(BytesStart::new("tab")))?;
                writer.write_event(Event::Text(BytesText::new(
                    &tab.to_file_value()?.to_string(),
                )))?;
                writer.write_event(Event::End(BytesEnd::new("tab")))?;
            }
            writer.write_event(Event::End(BytesEnd::new("tabs")))?;
        }

        writer.write_event(Event::End(BytesEnd::new("text_symbol")))?;

        Ok(())
    }
}
