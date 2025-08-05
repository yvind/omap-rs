#[derive(Debug, Clone)]
pub struct OmapVersion {
    xmlns: String,
    version: u8,
}

#[derive(Debug, Clone)]
pub struct XmlVersion {
    version: String,
    encoding: String,
}
