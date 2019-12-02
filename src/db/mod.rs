use diesel::expression::sql_literal::sql;
use diesel::prelude::*;
use diesel::sql_types::*;
use diesel::SqliteConnection;
use r2d2::{CustomizeConnection, Pool};
use r2d2_diesel::{self, ConnectionManager};

pub mod schema;

embed_migrations!();

#[derive(Debug)]
struct SqliteInitializer;

impl CustomizeConnection<SqliteConnection, r2d2_diesel::Error> for SqliteInitializer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), r2d2_diesel::Error> {
        sql::<Integer>("PRAGMA foreign_keys = ON")
            .execute(conn)
            .map_err(|x| r2d2_diesel::Error::QueryError(x))?;

        Ok(())
    }
}

pub fn create_pool(
    connection_string: impl Into<String>,
) -> Result<Pool<ConnectionManager<SqliteConnection>>, Box<dyn std::error::Error>> {
    let manager = ConnectionManager::<SqliteConnection>::new(connection_string);
    let pool = Pool::builder()
        .connection_customizer(Box::new(SqliteInitializer {}))
        .build(manager)?;

    embedded_migrations::run(&*pool.get()?)?;

    Ok(pool)
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(test)]
    pub fn test_connection() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:")
            .expect("SQLite should be able to create an in-memory database");

        SqliteInitializer.on_acquire(&mut conn).unwrap();
        embedded_migrations::run(&conn).unwrap();

        conn
    }

    #[test]
    fn smoke_test() {
        test_connection();
    }
}
