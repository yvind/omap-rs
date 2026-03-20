use geo_types::Coord;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::SymbolCommon;
use crate::{
    Code, Error, NonNegativeF64, Result,
    colors::{ColorSet, SymbolColor},
    utils::{self, try_get_attr_raw},
};

/// The framing mode for a text symbol.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Default)]
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
    pub fn get_id(&self) -> u8 {
        match self {
            FramingMode::NoFraming => 0,
            FramingMode::LineFraming(_) => 1,
            FramingMode::ShadowFraming(_) => 2,
        }
    }
}

/// Line-based framing (halo) around text characters.
#[derive(Debug, Clone)]
pub struct LineFraming {
    pub color: SymbolColor,
    pub framing_line_half_width: NonNegativeF64,
}

/// Shadow framing behind text characters.
#[derive(Debug, Clone)]
pub struct ShadowFraming {
    pub color: SymbolColor,
    pub shadow_offset: Coord<f64>,
}

/// A line drawn below the text (underline).
#[derive(Debug, Clone)]
pub struct LineBelow {
    pub color: SymbolColor,
    pub width: NonNegativeF64,
    pub distance: NonNegativeF64,
}

/// A text symbol definition.
#[derive(Debug, Clone)]
pub struct TextSymbol {
    /// The common symbol fields
    pub common: SymbolCommon,
    /// f.ex Arial
    pub font_family: String,
    /// Should not be more than 3 chars long
    pub icon_text: String,
    /// Color of the text
    pub color: SymbolColor,

    // OCD compatibility
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
    pub paragraph_spacing: f64, // in mm
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
    pub fn new(code: Code, name: String) -> TextSymbol {
        let common = SymbolCommon {
            code,
            name,
            ..Default::default()
        };
        TextSymbol {
            common,
            font_family: String::from("Arial"),
            icon_text: String::new(),
            color: SymbolColor::NoColor,
            custom_tabs: Vec::new(),
            line_below: None,
            line_spacing: NonNegativeF64::clamped_from(1.0),
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
    pub fn get_name(&self) -> &str {
        &self.common.name
    }

    pub(super) fn parse<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        color_set: &ColorSet,
        attributes: SymbolCommon,
    ) -> Result<TextSymbol> {
        let mut common = attributes;
        let mut icon_text = String::new();
        let mut is_rotatable = false;
        let mut font_family = String::from("Arial");
        let mut font_size = NonNegativeF64::clamped_from(4.0);
        let mut bold = false;
        let mut italic = false;
        let mut underline = false;
        let mut color = SymbolColor::NoColor;
        let mut line_spacing = NonNegativeF64::clamped_from(1.0);
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
                    b"description" => {
                        if let Event::Text(text) = reader.read_event_into(&mut buf)? {
                            common.description = String::from_utf8(text.to_vec())?;
                        }
                    }
                    b"text_symbol" => {
                        icon_text = try_get_attr_raw(&e, "icon_text").unwrap_or_default();
                        is_rotatable = try_get_attr_raw(&e, "rotatable").unwrap_or(false);
                    }
                    b"font" => {
                        font_family =
                            try_get_attr_raw(&e, "family").unwrap_or_else(|| String::from("Arial"));
                        let fs = try_get_attr_raw(&e, "size").unwrap_or(4000);
                        font_size = NonNegativeF64::from_file_value(fs);
                        bold = try_get_attr_raw(&e, "bold").unwrap_or(false);
                        italic = try_get_attr_raw(&e, "italic").unwrap_or(false);
                        underline = try_get_attr_raw(&e, "underline").unwrap_or(false);
                    }
                    b"text" => {
                        let ci = try_get_attr_raw(&e, "color").unwrap_or(-1);
                        color = SymbolColor::from_index(ci, color_set);
                        let ls = try_get_attr_raw(&e, "line_spacing").unwrap_or(1.0);
                        line_spacing = NonNegativeF64::clamped_from(ls);
                        paragraph_spacing = NonNegativeF64::from_file_value(
                            try_get_attr_raw(&e, "paragraph_spacing").unwrap_or(0),
                        )
                        .get();
                        character_spacing =
                            try_get_attr_raw(&e, "character_spacing").unwrap_or(0.0);
                        kerning = try_get_attr_raw(&e, "kerning").unwrap_or(false);
                    }
                    b"framing" => {
                        let fc = try_get_attr_raw(&e, "color").unwrap_or(-1);
                        let framing_color = SymbolColor::from_index(fc, color_set);
                        let mode = try_get_attr_raw(&e, "mode").unwrap_or(0);
                        framing_mode = Some(match mode {
                            1 => {
                                let half_width = NonNegativeF64::from_file_value(
                                    try_get_attr_raw(&e, "line_half_width").unwrap_or(0),
                                );
                                FramingMode::LineFraming(LineFraming {
                                    color: framing_color,
                                    framing_line_half_width: half_width,
                                })
                            }
                            2 => {
                                let sx = try_get_attr_raw(&e, "shadow_x_offset").unwrap_or(0);
                                let sy = try_get_attr_raw(&e, "shadow_y_offset").unwrap_or(0);
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
                    b"line_below" => {
                        let lc = try_get_attr_raw(&e, "color").unwrap_or(-1);
                        let lb_color = SymbolColor::from_index(lc, color_set);
                        let w = try_get_attr_raw(&e, "width").unwrap_or(0);
                        let d = try_get_attr_raw(&e, "distance").unwrap_or(0);
                        line_below = Some(LineBelow {
                            color: lb_color,
                            width: NonNegativeF64::from_file_value(w),
                            distance: NonNegativeF64::from_file_value(d),
                        });
                    }
                    b"tabs" => {
                        // Parse tab elements inside
                    }
                    b"tab" => {
                        // tab text content parsed below
                    }
                    _ => {}
                },
                Event::Text(text) => {
                    // Could be tab content
                    if let Ok(v) = str::from_utf8(text.as_ref())?.parse() {
                        custom_tabs.push(NonNegativeF64::from_file_value(v));
                    }
                }
                Event::Empty(e) => {
                    if e.local_name().as_ref() == b"icon"
                        && let Some(src) = try_get_attr_raw(&e, "src")
                    {
                        common.custom_icon = Some(src);
                    }
                }
                Event::End(e) => {
                    if e.local_name().as_ref() == b"symbol" {
                        break;
                    }
                }
                Event::Eof => {
                    return Err(Error::ParseOmapFileError(
                        "Unexpected EOF in TextSymbol parsing".to_string(),
                    ));
                }
                _ => {}
            }
        }

        Ok(TextSymbol {
            common,
            is_rotatable,
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
            bold,
            italic,
            underline,
            kerning,
        })
    }

    pub(super) fn write<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        color_set: &ColorSet,
        index: usize,
    ) -> Result<()> {
        let mut bs = BytesStart::new("symbol").with_attributes([
            ("type", "8"),
            ("code", self.common.code.to_string().as_str()),
            (
                "name",
                quick_xml::escape::escape(self.common.name.as_str()).as_ref(),
            ),
            ("id", index.to_string().as_str()),
        ]);
        if self.common.is_hidden {
            bs.push_attribute(("is_hidden", "true"));
        }
        if self.common.is_helper_symbol {
            bs.push_attribute(("is_helper_symbol", "true"));
        }
        if self.common.is_protected {
            bs.push_attribute(("is_protected", "true"));
        }
        if self.is_rotatable {
            bs.push_attribute(("is_rotatable", "true"));
        }
        writer.write_event(Event::Start(bs))?;

        if !self.common.description.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("description")))?;
            writer.write_event(Event::Text(BytesText::new(&self.common.description)))?;
            writer.write_event(Event::End(BytesEnd::new("description")))?;
        }

        let mut bs = BytesStart::new("text_symbol");
        bs.push_attribute(("icon_text", self.icon_text.as_str()));
        // Rotatable is implicit for text symbols (always true in modern files)
        bs.push_attribute(("rotatable", "true"));
        writer.write_event(Event::Start(bs))?;

        // font element
        let mut font = BytesStart::new("font");
        font.push_attribute(("family", self.font_family.as_str()));
        font.push_attribute(("size", self.font_size.to_file_value()?.to_string().as_str()));
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

        // text element
        let mut text = BytesStart::new("text");
        text.push_attribute((
            "color",
            self.color.get_priority(color_set).to_string().as_str(),
        ));
        text.push_attribute(("line_spacing", self.line_spacing.get().to_string().as_str()));
        let ps_file = utils::to_file_value(self.paragraph_spacing)?;
        text.push_attribute(("paragraph_spacing", ps_file.to_string().as_str()));
        text.push_attribute((
            "character_spacing",
            self.character_spacing.to_string().as_str(),
        ));
        if self.kerning {
            text.push_attribute(("kerning", "true"));
        }
        writer.write_event(Event::Empty(text))?;

        // framing
        if let Some(fm) = &self.framing_mode {
            match fm {
                FramingMode::NoFraming => {}
                FramingMode::LineFraming(lf) => {
                    let mut f = BytesStart::new("framing");
                    f.push_attribute((
                        "color",
                        lf.color.get_priority(color_set).to_string().as_str(),
                    ));
                    f.push_attribute(("mode", "1"));
                    f.push_attribute((
                        "line_half_width",
                        lf.framing_line_half_width
                            .to_file_value()?
                            .to_string()
                            .as_str(),
                    ));
                    writer.write_event(Event::Empty(f))?;
                }
                FramingMode::ShadowFraming(sf) => {
                    let mut f = BytesStart::new("framing");
                    f.push_attribute((
                        "color",
                        sf.color.get_priority(color_set).to_string().as_str(),
                    ));
                    f.push_attribute(("mode", "2"));
                    let shadow = utils::to_file_coords(sf.shadow_offset)?;
                    f.push_attribute(("shadow_x_offset", shadow.x.to_string().as_str()));
                    f.push_attribute(("shadow_y_offset", shadow.y.to_string().as_str()));
                    writer.write_event(Event::Empty(f))?;
                }
            }
        }

        // line_below
        if let Some(lb) = &self.line_below {
            let mut lbe = BytesStart::new("line_below");
            lbe.push_attribute((
                "color",
                lb.color.get_priority(color_set).to_string().as_str(),
            ));
            lbe.push_attribute(("width", lb.width.to_file_value()?.to_string().as_str()));
            lbe.push_attribute((
                "distance",
                lb.distance.to_file_value()?.to_string().as_str(),
            ));
            writer.write_event(Event::Empty(lbe))?;
        }

        // custom tabs
        if !self.custom_tabs.is_empty() {
            let mut tabs = BytesStart::new("tabs");
            tabs.push_attribute(("count", self.custom_tabs.len().to_string().as_str()));
            writer.write_event(Event::Start(tabs))?;
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

        if let Some(icon) = &self.common.custom_icon {
            writer.write_event(Event::Empty(
                BytesStart::new("icon").with_attributes([("src", icon.as_str())]),
            ))?;
        }
        writer.write_event(Event::End(BytesEnd::new("symbol")))?;
        Ok(())
    }
}
