use std::fmt::{self, Display};

pub struct Tag {
    key: String,
    value: String,
}

impl Tag {
    pub fn new(k: impl Into<String>, v: impl Into<String>) -> Self {
        Tag {
            key: k.into(),
            value: v.into(),
        }
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<t k=\"{}\">{}</t>", self.key, self.value)
    }
}
