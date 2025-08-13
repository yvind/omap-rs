use std::{fmt::Display, str::FromStr};

use quick_xml::events::BytesStart;

use crate::editor::{Error, Result};

#[derive(Debug, Clone)]
pub struct OmapVersion {
    xmlns: String,
    version: u8,
}

impl OmapVersion {
    pub(crate) fn parse(element: &BytesStart) -> Result<Self> {
        let mut xmlns = None;
        let mut version = None;

        for attr in element.attributes() {
            let attr = attr?;

            match attr.key.local_name().as_ref() {
                b"xmlns" => xmlns = Some(String::from_utf8(attr.value.into_owned())?),
                b"version" => {
                    version = Some(u8::from_str(std::str::from_utf8(attr.value.as_ref())?)?)
                }
                _ => (),
            }
        }

        if xmlns.is_some() && version.is_some() {
            Ok(OmapVersion {
                xmlns: xmlns.unwrap(),
                version: version.unwrap(),
            })
        } else {
            Err(Error::InvalidFormat(
                "Could not read Omap version".to_string(),
            ))
        }
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(
            format!(
                "<map xmlns=\"{}\" version=\"{}\">",
                self.xmlns, self.version
            )
            .as_bytes(),
        )?;
        Ok(())
    }
}

impl Default for OmapVersion {
    fn default() -> Self {
        Self {
            xmlns: String::from("http://openorienteering.org/apps/mapper/xml/v2"),
            version: 9,
        }
    }
}

#[derive(Debug, Clone)]
pub struct XmlVersion {
    version: String,
    encoding: Encoding,
}

#[derive(Debug, Clone, Copy)]
pub enum Encoding {
    Utf8,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Encoding::Utf8 => f.write_str("UTF-8"),
        }
    }
}

impl Default for XmlVersion {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            encoding: Encoding::Utf8,
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

impl XmlVersion {
    pub(crate) fn parse(decl: quick_xml::events::BytesDecl) -> Result<Self> {
        let version = String::from_utf8(decl.version()?.into_owned())?;
        let encoding = std::str::from_utf8(&decl.encoding().ok_or(
            Error::UnsupportedEncoding("No Encoding tag found".to_owned()),
        )??)?
        .parse::<Encoding>()?;

        Ok(XmlVersion { version, encoding })
    }

    pub(crate) fn write<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
        writer.write_all(
            format!(
                "<?xml version=\"{}\" encoding=\"{}\"?>",
                self.version, self.encoding
            )
            .as_bytes(),
        )?;
        Ok(())
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }
}
