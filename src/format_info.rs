use std::str::FromStr;

use quick_xml::{
    Writer, XmlVersion,
    events::{BytesDecl, BytesStart, Event},
};

use crate::utils::{parse_attr_raw, try_get_attr_raw};
use crate::{Error, Result};

/// The OMAP file format version.
#[derive(Debug, Clone)]
pub(crate) struct OmapVersion;

impl<'writer> OmapVersion {
    pub(crate) fn parse(element: &BytesStart<'_>) -> Result<()> {
        let xmlns = try_get_attr_raw(element, "xmlns");
        let version = try_get_attr_raw::<u8>(element, "version");

        if xmlns != Some("http://openorienteering.org/apps/mapper/xml/v2".to_string()) {
            return Err(Error::InvalidFormat(
                "Cannot not read Omap version".to_string(),
            ));
        }
        if version.is_none() {
            return Err(Error::InvalidFormat(
                "Cannot not read Omap version".to_string(),
            ));
        }
        if let Some(v) = version
            && v != 9_u8
        {
            return Err(Error::InvalidFormat(
                "Cannot not read Omap version".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) fn write<W: std::io::Write>(writer: &'writer mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Start(
            BytesStart::new("map").with_attributes(
                [
                    ("xmlns", "http://openorienteering.org/apps/mapper/xml/v2"),
                    ("version", "9"),
                ]
                .into_iter(),
            ),
        ))?;
        Ok(())
    }
}

/// The XML declaration (version and encoding).
#[derive(Debug, Clone)]
pub(crate) struct XmlDeclaration;

impl XmlDeclaration {
    pub(crate) fn parse(decl: BytesDecl<'_>) -> Result<()> {
        let version = decl.xml_version()?;
        if version != XmlVersion::Explicit1_0 {
            return Err(Error::InvalidFormat(format!(
                "The XML version {:?} is not supported",
                version
            )));
        }

        let _ = parse_attr_raw::<Encoding>(decl.encoding().ok_or(
            Error::UnsupportedEncoding("No Encoding tag found".to_owned()),
        )??)
        .ok_or(Error::UnsupportedEncoding(
            "No Encoding tag found".to_owned(),
        ))?;
        Ok(())
    }

    pub(crate) fn write<W: std::io::Write>(writer: &mut Writer<W>) -> Result<()> {
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
        Ok(())
    }
}

/// Supported XML encodings.
#[derive(Debug, Clone, Copy)]
enum Encoding {
    /// UTF-8 encoding.
    Utf8,
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
