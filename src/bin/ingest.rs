#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;

use std::path::{Path, PathBuf};

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use rayon::prelude::*;
use stopwatch::Stopwatch;
use structopt::StructOpt;

#[path = "../db/mod.rs"]
mod db;
use db::schema::*;

type RgbImageF32 = ImageBuffer<Rgb<f32>, Vec<f32>>;

include!(concat!(env!("OUT_DIR"), "/srgb.rs"));

fn srgb_to_linear(s: u8) -> f32 {
    SRGB_TO_LINEAR[s as usize]
}

#[allow(unused)]
fn linear_to_srgb_binary_search(l: f32) -> u8 {
    match SRGB_TO_LINEAR.binary_search_by(|x| x.partial_cmp(&l).unwrap()) {
        Ok(i) => i as u8,
        Err(i) => i as u8, // Not exactly right
    }
}

#[allow(unused)]
fn linear_to_srgb_lookup(l: f32) -> u8 {
    fn to_12_bits(f: f32) -> u32 {
        const MANTISSA: u32 = 0x00ffffff;

        let u = f.to_bits();
        let u = u & (MANTISSA >> 1); // Discard topmost bit to subtract 1
        let u = u >> (24 - 1 - 12); // Keep 12 remaining topmost bits

        u
    }

    match l + 1. {
        l if l < 1. => 0,
        l if l >= 2. => 255,
        l => LINEAR_TO_SRGB[to_12_bits(l) as usize],
    }
}

#[allow(unused)]
fn linear_to_srgb_calculate(l: f32) -> u8 {
    let l = match l {
        l if l < 0. => 0.,
        l if l > 1. => 1.,
        l if l < 0.0031308 => (l * 12.92),
        l => 1.055 * l.powf(1.0 / 2.4) - 0.055,
    };

    (l * 255.) as u8
}

fn linear_to_srgb(l: f32) -> u8 {
    linear_to_srgb_lookup(l)
}

fn px_linear_to_srgb(l: &Rgb<f32>) -> Rgb<u8> {
    let ch = l.channels();
    Rgb::from_channels(
        linear_to_srgb(ch[0]),
        linear_to_srgb(ch[1]),
        linear_to_srgb(ch[2]),
        0,
    )
}

fn image_srgb_to_linear(src: RgbImage) -> RgbImageF32 {
    let (width, height) = src.dimensions();
    let data = src.into_raw();

    let data: Vec<_> = data.into_par_iter().map(|x| srgb_to_linear(x)).collect();

    RgbImageF32::from_raw(width, height, data).unwrap()
}

fn image_linear_to_srgb(src: RgbImageF32) -> RgbImage {
    let (width, height) = src.dimensions();
    let data = src.into_raw();

    let data: Vec<_> = data.into_par_iter().map(|x| linear_to_srgb(x)).collect();

    RgbImage::from_raw(width, height, data).unwrap()
}

fn avg_color(src: &RgbImageF32) -> Rgb<f32> {
    let acc = src
        .pixels()
        .fold(Rgb::from_channels(0., 0., 0., 0.), |acc, x| {
            acc.map2(&x, |a, b| a + b)
        });
    let pixels = (src.width() * src.height()) as f32;

    acc.map(|x| x / pixels)
}

fn encode_jpeg(img: RgbImage, quality: u8) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut encoder = image::jpeg::JPEGEncoder::new_with_quality(&mut buf, quality);
    let (width, height) = (img.width(), img.height());
    encoder.encode(&img.into_raw(), width, height, image::ColorType::RGB(8))?;
    Ok(buf)
}

fn ingest(
    file: impl AsRef<Path>,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
) -> Result<i32, Box<dyn std::error::Error>> {
    let sw = Stopwatch::start_new();
    let img = image::open(file.as_ref())?.to_rgb();
    eprintln!(
        "ORG: Decoded original jpeg {}x{} in {}ms",
        img.width(),
        img.height(),
        sw.elapsed_ms()
    );

    let sw = Stopwatch::start_new();
    let img = image_srgb_to_linear(img.clone());
    eprintln!(
        "ORG: Converted original to linear color space in {}ms",
        sw.elapsed_ms()
    );

    // To consider: Always store the original, to be able to render new sizes?
    // Also: To be able to order photo prints based on collections in pixu.rs?

    let large = if img.width() > 2560 {
        let sw = Stopwatch::start_new();
        let nwidth = 2560;
        let nheight = nwidth * img.height() / img.width();
        let img = image::imageops::resize(&img, nwidth, nheight, image::imageops::Lanczos3);
        eprintln!(
            "LRG: Downscaled to {}x{} in {}ms",
            nwidth,
            nheight,
            sw.elapsed_ms()
        );
        img
    } else {
        img
    };

    let (large_jpeg, r2) = rayon::join(
        || -> Result<Vec<u8>, std::io::Error> {
            let sw = Stopwatch::start_new();
            let large_srgb = image_linear_to_srgb(large.clone());
            eprintln!("LRG: Converted to sRGB in {}ms", sw.elapsed_ms());

            let sw = Stopwatch::start_new();
            let large_jpeg = encode_jpeg(large_srgb, 80)?;
            eprintln!(
                "LRG: Encoded as JPEG in {}ms (proof: {})",
                sw.elapsed_ms(),
                large_jpeg[100]
            );

            // TODO Store large_srgb. In the mean time, print value to foil optimizer

            Ok(large_jpeg)
        },
        || -> Result<_, std::io::Error> {
            let sw = Stopwatch::start_new();
            let nwidth = 320;
            let nheight = nwidth * large.height() / large.width();
            let small = image::imageops::resize(&large, nwidth, nheight, image::imageops::Lanczos3);
            eprintln!(
                "SML: Downscaled to {}x{} in {}ms",
                nwidth,
                nheight,
                sw.elapsed_ms()
            );

            let (small_jpeg, col) = rayon::join(
                || -> Result<_, std::io::Error> {
                    let sw = Stopwatch::start_new();
                    let small_srgb = image_linear_to_srgb(small.clone());
                    let small_jpeg = encode_jpeg(small_srgb, 20)?;
                    // TODO Store small_srgb
                    // Use value, for benchmarking:
                    eprintln!(
                        "SML: Converted and encoded as JPEG in {}ms (proof: {})",
                        sw.elapsed_ms(),
                        small_jpeg[100]
                    );

                    Ok(small_jpeg)
                },
                || {
                    let sw = Stopwatch::start_new();
                    let col = px_linear_to_srgb(&avg_color(&small));
                    // TODO Store col
                    // Use value, for benchmarking
                    let ch = col.channels();
                    eprintln!(
                        "AVG: Found average color in {}ms: rgb({}, {}, {})",
                        sw.elapsed_ms(),
                        ch[0],
                        ch[1],
                        ch[2]
                    );

                    col
                },
            );

            Ok((small_jpeg?, col))
        },
    );

    let large_jpeg = large_jpeg?;
    let (small_jpeg, col) = r2?;

    use diesel::expression::sql_literal::sql;

    let db_connection = db_pool.get()?;
    db_connection.transaction(|| {
        #[derive(Insertable)]
        #[table_name = "thumbs"]
        struct Thumb<'a> {
            media_type: &'a str,
            data: &'a [u8],
        }

        diesel::insert_into(thumbs::table)
            .values(&Thumb {
                media_type: "image/jpeg",
                data: &small_jpeg,
            })
            .execute(&*db_connection)?;

        let thumbs_id = sql::<(diesel::sql_types::Integer)>("SELECT LAST_INSERT_ROWID()")
            .load::<i32>(&*db_connection)?
            .pop()
            .expect("Statement must evaluate to an integer");

        #[derive(Insertable)]
        #[table_name = "pixurs"]
        struct Pixur {
            average_color: i32,
            thumbs_id: i32,
        }

        diesel::insert_into(pixurs::table)
            .values(&Pixur {
                average_color: col.channels()[0] as i32
                    + ((col.channels()[1] as i32) << 8)
                    + ((col.channels()[2] as i32) << 16),
                thumbs_id,
            })
            .execute(&*db_connection)?;

        let pixurs_id = sql::<(diesel::sql_types::Integer)>("SELECT LAST_INSERT_ROWID()")
            .load::<i32>(&*db_connection)?
            .pop()
            .expect("Statement must evaluate to an integer");

        #[derive(Insertable)]
        #[table_name = "images"]
        struct Image<'a> {
            media_type: &'a str,
            data: &'a [u8],
        }

        diesel::insert_into(images::table)
            .values(&Image {
                media_type: "image/jpeg",
                data: &large_jpeg,
            })
            .execute(&*db_connection)?;

        let images_id = sql::<(diesel::sql_types::Integer)>("SELECT LAST_INSERT_ROWID()")
            .load::<i32>(&*db_connection)?
            .pop()
            .expect("Statement must evaluate to an integer");

        #[derive(Insertable)]
        #[table_name = "images_meta"]
        struct ImageMeta {
            id: i32,
            width: i32,
            height: i32,
            pixurs_id: i32,
        }

        diesel::insert_into(images_meta::table)
            .values(&ImageMeta {
                id: images_id,
                width: large.width() as i32,
                height: large.height() as i32,
                pixurs_id,
            })
            .execute(&*db_connection)?;

        Ok(pixurs_id)
    })
}

#[derive(StructOpt)]
struct Params {
    /// SQLite database file
    #[structopt(long = "db", name = "DB")]
    db: String,

    files: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Params::from_args();
    let db_pool = db::create_pool(args.db)?;

    let mut ok = true;

    for file in args.files {
        let sw = Stopwatch::start_new();
        match ingest(&file, db_pool.clone()) {
            Ok(id) => eprintln!(
                "Ingested {} in {}ms as ID {}",
                file.display(),
                sw.elapsed_ms(),
                id
            ),
            Err(err) => {
                ok = false;
                eprintln!("Error ingesting {}: {}", file.display(), err);
            }
        }
    }

    std::process::exit(if ok { 0 } else { 1 });
}
