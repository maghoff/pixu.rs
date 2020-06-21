use diesel::prelude::*;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;

use super::auth;
use crate::db::schema::*;

// Includes a private field to make construction private to this module
pub struct CanEdit(());

pub struct CanEditProvider {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl auth::authorizer::Provider for CanEditProvider {
    type Authorization = CanEdit;

    fn get_authorization(&self, sub: &str) -> Result<Option<Self::Authorization>, web::Error> {
        use diesel::dsl::*;

        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| web::Error::InternalServerError)?;

        let authorized: bool = select(exists(uploaders::table.filter(uploaders::sub.eq(sub))))
            .first::<bool>(&*db_connection)
            .expect("Query must return 1 result");

        if authorized {
            Ok(Some(CanEdit(())))
        } else {
            Ok(None)
        }
    }
}
