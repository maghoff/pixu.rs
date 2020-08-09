use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Error, MediaType, RepresentationBox, Resource, Response};

use super::auth;
use super::handling_error::HandlingError;
use crate::db::schema::*;
use crate::id30::Id30;

pub struct Pixu {
    title: String,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: Id30,
}

struct Photo {
    average_color: String,
    thumb_url: String,
    large_url: String,

    height: &'static str,
    max_height: Option<String>,
    max_width: Option<String>,

    background_position: String,

    comment: Option<String>,
    comment_position: String,
}

#[derive(BartDisplay)]
#[template = "templates/pixu.html"]
struct Get<'a> {
    top_color: &'a str,
    bottom_color: &'a str,

    photos: &'a [Photo],
}

#[derive(Queryable)]
#[allow(unused)]
struct PixurSeries {
    id: i32,
    order: i32,
    pixur_id: i32,
}

#[derive(Queryable)]
struct Pixurs {
    #[allow(unused)]
    id: Id30,
    average_color: i32,
    thumbs_id: Id30,

    #[allow(unused)]
    created: chrono::NaiveDateTime,

    image_aspect_ratio: f32,

    crop_left: f32,
    crop_right: f32,
    crop_top: f32,
    crop_bottom: f32,

    comment: Option<String>,
    comment_position: String, // TODO Could be an enum type instead (top, center, bottom)
}

fn photo_from_pixurs(
    pix: Pixurs,
    large_id: Id30,
    vh_height: f32,
    vh_height_str: &'static str,
) -> Result<Photo, HandlingError> {
    let aspect = pix.image_aspect_ratio;

    let crop_width = pix.crop_right - pix.crop_left;
    let crop_height = pix.crop_bottom - pix.crop_top;

    // Some of the following calculations are prone to division by zero
    // for edge cases. Define EPSILON as the limit of when to care.
    const EPSILON: f32 = 1. / 10_000.;

    // For a target width, defined by crop_width, calculate the
    // corresponding height in terms of available width:
    let max_height = if crop_width > EPSILON {
        Some(format!("{:.2}vw", 100. / aspect / crop_width))
    } else {
        None
    };

    // The dual calculation to the above, switching width and height
    let max_width = if crop_height > EPSILON {
        Some(format!("{:.2}vh", vh_height * aspect / crop_height))
    } else {
        None
    };

    // Calculate the ratio of how much is cropped away on the top/left side
    // vs how much is cropped away vertically/horizontally in total
    let horizontal_crop = pix.crop_left + 1. - pix.crop_right;
    let background_position_x = if horizontal_crop > EPSILON {
        100. * (pix.crop_left / horizontal_crop)
    } else {
        50.
    };

    let vertical_crop = pix.crop_top + 1. - pix.crop_bottom;
    let background_position_y = if vertical_crop > EPSILON {
        100. * (pix.crop_top / vertical_crop)
    } else {
        50.
    };

    let background_position = format!(
        "{:.2}% {:.2}%",
        background_position_x, background_position_y
    );

    Ok(Photo {
        average_color: format!("#{:06x}", pix.average_color),
        thumb_url: format!("thumb/{}", pix.thumbs_id),
        large_url: format!("img/{}", large_id),
        height: vh_height_str,
        max_height,
        max_width,
        background_position,
        comment: pix.comment,
        comment_position: pix.comment_position,
    })
}

impl Pixu {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        // TODO Schedule IO operations on some kind of background thread (schedule_blocking)
        // TODO Parallelize independent queries

        // Slight inefficiency: Reading out unnecessary PixurSeries objects
        let pix: Vec<(PixurSeries, Pixurs)> = pixur_series::table
            .inner_join(pixurs::table)
            .filter(pixur_series::id.eq(self.id))
            .order(pixur_series::order.asc())
            .load(&*db_connection)
            .map_err(|_| HandlingError::InternalServerError)?;

        let (vh_height, vh_height_str) = match pix.len() {
            0 => Err(HandlingError::InternalServerError)?, // Should have yielded 404 earlier on
            1 => (100., "100vh"),
            _ => (97., "97vh"),
        };

        let photos = pix
            .into_iter()
            .map(|(_, pix)| {
                // TODO Consolidate to one big query in parent scope, to avoid running O(n) queries
                // TODO Load all sizes of images and allow client side to pick the best size
                let large_id: Id30 = images_meta::table
                    .filter(images_meta::pixurs_id.eq(pix.id))
                    .order(images_meta::width.desc())
                    .select(images_meta::id)
                    .first(&*db_connection)
                    .map_err(|_| HandlingError::InternalServerError)?;

                photo_from_pixurs(pix, large_id, vh_height, vh_height_str)
            })
            .collect::<Result<Vec<_>, HandlingError>>()?;

        Ok(Response::new(
            web::Status::Ok,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        super::Layout {
                            title: &self.title,
                            body: &Get {
                                top_color: &photos.first().unwrap().average_color,
                                bottom_color: &photos.last().unwrap().average_color,
                                photos: &photos,
                            },
                        }
                        .to_string(),
                    ) as RepresentationBox
                }),
            )],
        ))
    }
}

#[async_trait::async_trait]
impl web::Get for Pixu {
    async fn representations(self: Box<Self>) -> Response {
        let title = self.title.clone();

        self.try_get().await.unwrap_or_else(|e| e.render(&title))
    }
}

pub struct AuthorizationConsumer {
    pub title: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl auth::authorizer::Consumer for AuthorizationConsumer {
    type Authorization = Id30;

    fn authorization<'a>(self, id: Id30) -> Result<Resource, Error> {
        Ok(Resource {
            etag: None,
            get: Some(Box::new(Pixu {
                title: self.title,
                db_pool: self.db_pool,
                id,
            })),
            post: None,
        })
    }
}

pub struct AuthorizationProvider {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub id: Id30,
}

impl auth::authorizer::Provider for AuthorizationProvider {
    type Authorization = Id30;

    fn get_authorization(&self, sub: &str) -> Result<Option<Self::Authorization>, Error> {
        use diesel::dsl::*;

        let db_connection = self.db_pool.get().map_err(|_| Error::InternalServerError)?;

        let is_uploader = select(exists(uploaders::table.filter(uploaders::sub.eq(sub))))
            .first(&*db_connection)
            .expect("Query must return 1 result");

        let authorized = is_uploader
            || select(exists(
                pixur_series_authorizations::table
                    .filter(pixur_series_authorizations::pixur_series_id.eq(self.id))
                    .filter(pixur_series_authorizations::sub.eq(sub)),
            ))
            .first(&*db_connection)
            .expect("Query must return 1 result");

        if authorized {
            Ok(Some(self.id))
        } else {
            Ok(None)
        }
    }
}
