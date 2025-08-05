#[derive(Debug, Clone)]
pub(super) struct Barrier {
    version: u8,
    required: String,
}

impl Barrier {
    pub(super) fn write<W: std::io::Write>(self, write: &mut W) -> Result<(), std::io::Error> {
        write.write_all(
            format!(
                "<barrier version=\"{}\" required=\"{}\">\n",
                self.version, self.required
            )
            .as_bytes(),
        )
    }
}
