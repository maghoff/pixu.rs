use byteorder::ByteOrder;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use diesel::sqlite::Sqlite;
use rand::Rng;
use std::fmt;
use std::io::Write;
use std::str::FromStr;

/// 30 bit integral identifier
#[derive(PartialEq, Eq, Clone, Copy, Debug, AsExpression, FromSqlRow)]
#[sql_type = "Integer"]
pub struct Id30(u32);

impl Id30 {
    pub fn new_random(rng: &mut (impl Rng + ?Sized)) -> Id30 {
        Id30::from(rng.gen_range(0, 0x4000_0000))
    }
}

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

impl ToSql<Integer, Sqlite> for Id30 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        ToSql::<Integer, Sqlite>::to_sql(&(self.0 as i32), out)
    }
}

impl FromSql<Integer, Sqlite> for Id30 {
    fn from_sql(value: Option<&<Sqlite as Backend>::RawValue>) -> deserialize::Result<Self> {
        let num = value.ok_or("Unexpected NULL")?.read_integer() as u32;
        if num & 0xC000_0000 != 0 {
            return Err(format!("Value out of range, {} does not fit into 30 bits", num).into());
        }
        Ok(Id30::from(num))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use diesel::prelude::*;
    use diesel::sql_query;
    use std::error::Error;

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

    #[test]
    fn basic_db_roundtrip() -> Result<(), Box<dyn Error>> {
        let conn = SqliteConnection::establish(":memory:")?;

        #[derive(QueryableByName, PartialEq, Eq, Debug)]
        struct Row {
            #[sql_type = "Integer"]
            id30: Id30,
        }

        let res = sql_query("SELECT ? as id30")
            .bind::<Integer, _>(Id30::from(0x1234_5678))
            .load::<Row>(&conn)?;

        assert_eq!(
            &[Row {
                id30: Id30::from(0x1234_5678)
            }],
            res.as_slice()
        );

        Ok(())
    }

    #[test]
    fn db_invalid_value_gives_error() -> Result<(), Box<dyn Error>> {
        let conn = SqliteConnection::establish(":memory:")?;

        #[derive(QueryableByName, PartialEq, Eq, Debug)]
        struct Row {
            #[sql_type = "Integer"]
            id30: Id30,
        }

        let res = sql_query("SELECT 0x12345678 as id30").load::<Row>(&conn);
        assert!(res.is_ok());

        let res = sql_query("SELECT 0x7fffffff as id30").load::<Row>(&conn);
        assert!(res.is_err());

        Ok(())
    }
}
