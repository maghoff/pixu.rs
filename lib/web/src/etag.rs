use std::fmt;

// String? Really? Maybe Cow or something instead?
pub enum ETag {
    Weak(String),
    Strong(String),
}

impl fmt::Display for ETag {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // TODO Escape. Better typing for validating ETags? (IntoETag?)
        // Reference for ETag grammar: https://stackoverflow.com/a/11572348
        match self {
            ETag::Weak(tag) => write!(fmt, "W/\"{}\"", tag),
            ETag::Strong(tag) => write!(fmt, "\"{}\"", tag),
        }
    }
}
