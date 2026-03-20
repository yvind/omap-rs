use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use crate::Result;

pub(super) fn parse<R: std::io::BufRead>(reader: &mut Reader<R>) -> Result<String> {
    let mut notes = String::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Text(bytes_text) => {
                notes.push_str(&bytes_text.xml_content()?);
            }
            Event::GeneralRef(bytes_ref) => {
                notes.push_str(&quick_xml::escape::unescape(&format!(
                    "&{};",
                    &bytes_ref.xml_content()?
                ))?);
            }
            _ => break,
        }
    }
    Ok(notes)
}

pub(super) fn write<W: std::io::Write>(notes: &str, writer: &mut Writer<W>) -> Result<()> {
    writer.write_event(Event::Start(BytesStart::new("notes")))?;
    writer.write_event(Event::Text(BytesText::new(notes)))?;
    writer.write_event(Event::End(BytesEnd::new("notes")))?;
    Ok(())
}
