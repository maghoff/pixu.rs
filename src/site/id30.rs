use byteorder::ByteOrder;
use std::fmt;
use std::str::FromStr;

/// 30 bit integral identifier
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Id30(u32);

impl From<u32> for Id30 {
    fn from(num: u32) -> Self {
        debug_assert_eq!(num & 0xC000_0000, 0);
        Id30(num & 0x3FFF_FFFF)
    }
}

impl From<Id30> for u32 {
    fn from(id: Id30) -> Self {
        id.0
    }
}

impl fmt::Display for Id30 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; 5];
        byteorder::LittleEndian::write_u32(&mut buf, self.0);
        buf[3] = buf[3] << 2;
        let mut s = base32::encode(base32::Alphabet::Crockford, &buf);
        s.truncate(6);
        s.make_ascii_lowercase();
        write!(f, "{}", s)
    }
}

impl FromStr for Id30 {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 6 {
            return Err(());
        }

        let mut text = text.to_string();
        text.push_str("00");
        let mut buf = base32::decode(base32::Alphabet::Crockford, &text).ok_or(())?;
        buf[3] = buf[3] >> 2;

        Ok(Id30(byteorder::LittleEndian::read_u32(&buf)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn roundtrip_u32() {
        let x = Id30::from(0x1234_5678);
        assert_eq!(u32::from(x), 0x1234_5678);
    }

    #[test]
    fn roundtrip_str() {
        let x: Id30 = "abcdef".parse().unwrap();
        assert_eq!(x.to_string(), "abcdef");
    }

    #[test]
    fn to_string() {
        let x = Id30::from(0x3FFF_FFFF);
        assert_eq!(x.to_string(), "zzzzzz");

        let x = Id30::from(0);
        assert_eq!(x.to_string(), "000000");
    }

    #[test]
    fn parse() {
        let x: Id30 = "zzzzzz".parse().unwrap();
        assert_eq!(x, Id30::from(0x3FFF_FFFF));

        let x: Id30 = "000000".parse().unwrap();
        assert_eq!(x, Id30::from(0));
    }
}
