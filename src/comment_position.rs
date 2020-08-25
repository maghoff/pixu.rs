use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use serde_derive::{Serialize, Deserialize};
use std::io::Write;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[sql_type = "Text"]
pub enum CommentPosition {
    Top,
    Center,
    Bottom,
}

serde_plain::forward_display_to_serde!(CommentPosition);
serde_plain::forward_from_str_to_serde!(CommentPosition);

impl ToSql<Text, Sqlite> for CommentPosition {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        let s = match self {
            CommentPosition::Top => "top",
            CommentPosition::Center => "center",
            CommentPosition::Bottom => "bottom",
        };
        ToSql::<Text, Sqlite>::to_sql(s, out)
    }
}

impl FromSql<Text, Sqlite> for CommentPosition {
    fn from_sql(value: Option<&<Sqlite as Backend>::RawValue>) -> deserialize::Result<Self> {
        // See Diesel's documentation on how to implement FromSql for Sqlite,
        // especially with regards to the unsafe conversion below.
        // http://docs.diesel.rs/diesel/deserialize/trait.FromSql.html
        let text_ptr = <*const str as FromSql<Text, Sqlite>>::from_sql(value)?;
        let text = unsafe { &*text_ptr };
        match text {
            "top" => Ok(CommentPosition::Top),
            "center" => Ok(CommentPosition::Center),
            "bottom" => Ok(CommentPosition::Bottom),
            _ => Err("Invalid value in database".into()),
        }
    }
}
