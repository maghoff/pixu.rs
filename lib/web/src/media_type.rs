use std::fmt;

// FIXME Rework entire module. Is there a reusable media type type in the ecosystem?

// FIXME Very alloc heavy struct
// FIXME Verify validity of data on creation
pub struct MediaType {
    pub type_category: String,
    pub subtype: String,
    pub args: Vec<String>,
}

impl MediaType {
    pub fn new(
        type_category: impl ToString,
        subtype: impl ToString,
        args: impl Into<Vec<String>>,
    ) -> MediaType {
        MediaType {
            type_category: type_category.to_string(),
            subtype: subtype.to_string(),
            args: args.into(),
        }
    }

    pub fn parse(src: &str) -> MediaType {
        let parts = src.splitn(2, '/').collect::<Vec<_>>();
        MediaType {
            type_category: parts[0].to_string(),
            subtype: parts[1].to_string(),
            args: vec![],
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // FIXME: Will willingly generate invalid media type strings if the
        // components are invalid

        write!(fmt, "{}/{}", self.type_category, self.subtype)?;

        for (i, arg) in self.args.iter().enumerate() {
            if i == 0 {
                write!(fmt, ";")?;
            } else {
                write!(fmt, "&")?;
            }
            write!(fmt, "{}", arg)?;
        }

        Ok(())
    }
}
