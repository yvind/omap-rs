use std::{fmt::Display, str::FromStr};

use quick_xml::{
    Writer,
    events::{BytesDecl, BytesStart, Event},
};

use crate::utils::{parse_attr_raw, try_get_attr_raw};
use crate::{Code, Error, Result};

/// The OMAP file format version.
#[derive(Debug, Clone)]
pub struct OmapVersion {
    xmlns: String,
    version: u8,
}

impl<'writer> OmapVersion {
    pub(crate) fn parse(element: &BytesStart<'_>) -> Result<Self> {
        let xmlns = try_get_attr_raw(element, "xmlns");
        let version = try_get_attr_raw(element, "version");

        if let Some(xmlns) = xmlns
            && let Some(version) = version
        {
            Ok(OmapVersion { xmlns, version })
        } else {
            Err(Error::InvalidFormat(
                "Could not read Omap version".to_string(),
            ))
        }
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &'writer mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Start(
            BytesStart::new("map").with_attributes(
                [
                    ("xmlns", self.xmlns.as_str()),
                    ("version", self.version.to_string().as_str()),
                ]
                .into_iter(),
            ),
        ))?;
        Ok(())
    }
}

impl Default for OmapVersion {
    fn default() -> Self {
        Self {
            xmlns: "http://openorienteering.org/apps/mapper/xml/v2".into(),
            version: 9,
        }
    }
}

/// The XML declaration (version and encoding).
#[derive(Debug, Clone)]
pub struct XmlVersion {
    version: String,
    encoding: Encoding,
}

impl Default for XmlVersion {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            encoding: Encoding::Utf8,
        }
    }
}

impl XmlVersion {
    pub(crate) fn parse(decl: BytesDecl<'_>) -> Result<Self> {
        let version = String::from_utf8(decl.version()?.into_owned())?;
        let encoding = parse_attr_raw(decl.encoding().ok_or(Error::UnsupportedEncoding(
            "No Encoding tag found".to_owned(),
        ))??)
        .ok_or(Error::UnsupportedEncoding(
            "No Encoding tag found".to_owned(),
        ))?;
        Ok(XmlVersion { version, encoding })
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Decl(BytesDecl::new(
            self.version.as_str(),
            Some(self.encoding.as_ref()),
            None,
        )))?;
        Ok(())
    }

    /// Get the XML version string (e.g. `"1.0"`).
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the encoding used in the XML declaration.
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }
}

/// Supported XML encodings.
#[derive(Debug, Clone, Copy, Default)]
pub enum Encoding {
    /// UTF-8 encoding.
    #[default]
    Utf8,
}

impl AsRef<str> for Encoding {
    fn as_ref(&self) -> &str {
        match self {
            Encoding::Utf8 => "UTF-8",
        }
    }
}
impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Encoding::Utf8 => f.write_str("UTF-8"),
        }
    }
}

impl FromStr for Encoding {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "UTF-8" | "utf-8" | "Utf-8" => Ok(Encoding::Utf8),
            _ => Err(Error::UnsupportedEncoding(s.to_string())),
        }
    }
}

/// A compatibility barrier declaring the minimum reader version.
#[derive(Debug, Clone)]
pub struct Barrier {
    /// The barrier version number.
    pub version: u8,
    /// The minimum required reader version.
    pub required: Code,
    /// Whether to skip unknown data beyond the barrier.
    pub skip: bool,
}

impl Default for Barrier {
    fn default() -> Self {
        Self {
            version: 6,
            required: Code {
                major: 0,
                minor: 6,
                patch: 0,
            },
            skip: false,
        }
    }
}

impl Barrier {
    pub(crate) fn parse(element: &BytesStart<'_>) -> Result<Self> {
        let mut skip = false;
        let mut required = Code::default();
        let mut version = 0;
        for attr in element.attributes().filter_map(std::result::Result::ok) {
            match attr.key.local_name().as_ref() {
                b"version" => version = parse_attr_raw(attr.value).unwrap_or(version),
                b"required" => required = parse_attr_raw(attr.value).unwrap_or(required),
                b"action" => skip = attr.value.as_ref() == b"skip",
                _ => (),
            }
        }
        if required == Code::default() || version == 0 {
            Err(Error::ParseOmapFileError("Bad barrier".to_string()))
        } else {
            Ok(Barrier {
                version,
                required,
                skip,
            })
        }
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        let mut bytes_start = BytesStart::new("barrier").with_attributes([
            ("version", format!("{}", self.version).as_str()),
            ("required", format!("{}", self.required).as_str()),
        ]);
        if self.skip {
            bytes_start.push_attribute(("action", "skip"));
        }
        writer.write_event(Event::Start(bytes_start))?;
        Ok(())
    }
}
