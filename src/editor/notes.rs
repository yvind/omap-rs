use crate::editor::{Error, Result};
use std::io::{BufWriter, Write};
use quick_xml::Reader;

pub(super) fn parse<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<String> {
    let mut notes = String::new();

    let mut buf = Vec::new();

    loop {
        let event = reader.read_event_into(&mut buf)?;

        match event {
            quick_xml::events::Event::End(_) => break,
            quick_xml::events::Event::Text(bytes_text) => {
                notes.push_str(&bytes_text.decode()?);
            }
            quick_xml::events::Event::GeneralRef(bytes_ref) => {
                notes.push_str(&bytes_ref.decode()?);
            }
            _ => return Err(Error::MapPartMergeError),
        }
    }
    Ok(notes)
}

pub(super) fn write<W: std::io::Write>(notes: &str, writer: &mut BufWriter<W>) -> Result<()> {
        writer.write_all(
            format!(
                "<notes>{}</notes>\n",
                quick_xml::escape::escape(notes)
            )
            .as_bytes(),
        )?;
        Ok(())
}