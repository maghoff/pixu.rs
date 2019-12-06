use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use rayon::prelude::*;
use std::convert::TryInto;
use stopwatch::Stopwatch;

use crate::db::schema::*;
use crate::id30::Id30;

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

fn transform_by_orientation(img: image::RgbImage, orientation: u32) -> image::RgbImage {
    fn matmul(a: &[i32; 4], b: &[i32; 4]) -> [i32; 4] {
        [
            a[0] * b[0] + a[1] * b[2],
            a[0] * b[1] + a[1] * b[3],
            a[2] * b[0] + a[3] * b[2],
            a[2] * b[1] + a[3] * b[3],
        ]
    }

    fn vecmat(v: [i32; 2], mat: &[i32; 4]) -> [i32; 2] {
        [v[0] * mat[0] + v[1] * mat[2], v[0] * mat[1] + v[1] * mat[3]]
    }

    fn to_index(v: [i32; 2], mat: &[i32; 4], old_dim: &[i32; 2], new_dim: &[i32; 2]) -> isize {
        // Subtract 1 to calculate with the range of valid indices 0..=(dim-1)
        let od = [old_dim[0] - 1, old_dim[1] - 1];
        let nd = [new_dim[0] - 1, new_dim[1] - 1];

        let v = vecmat([v[0] * 2 - od[0], v[1] * 2 - od[1]], mat);
        ((v[0] + nd[0]) / 2 + ((v[1] + nd[1]) / 2) * new_dim[0]) as isize
    }

    if orientation == 0 {
        return img;
    }

    let mut t = [1, 0, 0, 1];
    let old_dim: [i32; 2] = [
        img.width().try_into().unwrap(),
        img.height().try_into().unwrap(),
    ];

    let new_dim = if orientation & 0b100 != 0 {
        [old_dim[1], old_dim[0]]
    } else {
        old_dim
    };

    if orientation & 0b100 != 0 {
        t = matmul(&t, &[0, 1, 1, 0]);
    }

    if orientation & 0b010 != 0 {
        t = matmul(&t, &[-1, 0, 0, -1]);
    }

    if orientation & 0b001 != 0 {
        t = matmul(&t, &[-1, 0, 0, 1]);
    }

    let mut dest_buf = vec![127u8; (new_dim[0] * new_dim[1] * 3) as usize];
    let mut dest = to_index([0, 0], &t, &old_dim, &new_dim) * 3;
    let dest_dy = to_index([0, 1], &t, &old_dim, &new_dim) * 3 - dest;
    let dest_dx = to_index([1, 0], &t, &old_dim, &new_dim) * 3 - dest;

    let px = img.into_raw();
    let mut px_iter = px.into_iter();
    for _y in 0..old_dim[1] {
        let mut row_dest = dest;
        for _x in 0..old_dim[0] {
            dest_buf[row_dest as usize + 0] = px_iter.next().unwrap();
            dest_buf[row_dest as usize + 1] = px_iter.next().unwrap();
            dest_buf[row_dest as usize + 2] = px_iter.next().unwrap();
            row_dest += dest_dx;
        }

        dest += dest_dy;
    }

    image::RgbImage::from_vec(new_dim[0] as _, new_dim[1] as _, dest_buf).unwrap()
}

pub fn ingest_jpeg(
    jpeg: &[u8],
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
) -> Result<Id30, Box<dyn std::error::Error>> {
    let sw = Stopwatch::start_new();
    let img = image::load_from_memory_with_format(jpeg, image::ImageFormat::JPEG)?.to_rgb();
    eprintln!(
        "ORG: Decoded original jpeg {}x{} in {}ms",
        img.width(),
        img.height(),
        sw.elapsed_ms()
    );

    let orientation = exif::Reader::new(&mut std::io::Cursor::new(jpeg))
        .ok()
        .as_ref()
        .and_then(|reader| reader.get_field(exif::Tag::Orientation, false))
        .and_then(|x| x.value.get_uint(0))
        .unwrap_or(1)
        - 1;
    eprintln!("ORG: Orientation: {:?}", orientation);

    let img = transform_by_orientation(img, orientation);

    let sw = Stopwatch::start_new();
    let img = image_srgb_to_linear(img);
    eprintln!(
        "ORG: Converted original to linear color space in {}ms",
        sw.elapsed_ms()
    );

    // TODO Consider: Always store the original, to be able to render new sizes?
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
                "LRG: Encoded as JPEG in {}ms, {}b",
                sw.elapsed_ms(),
                large_jpeg.len()
            );

            Ok(large_jpeg)
        },
        || -> Result<_, std::io::Error> {
            let sw = Stopwatch::start_new();
            let nwidth = 160;
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
                    eprintln!(
                        "SML: Converted and encoded as JPEG in {}ms, {}b",
                        sw.elapsed_ms(),
                        small_jpeg.len()
                    );

                    Ok(small_jpeg)
                },
                || {
                    let sw = Stopwatch::start_new();
                    let col = px_linear_to_srgb(&avg_color(&small));
                    eprintln!("AVG: Found average color in {}ms", sw.elapsed_ms());

                    col
                },
            );

            Ok((small_jpeg?, col))
        },
    );

    let large_jpeg = large_jpeg?;
    let (small_jpeg, col) = r2?;

    let db_connection = db_pool.get()?;
    db_connection.transaction(|| {
        use rand::{rngs::SmallRng, SeedableRng};

        let mut rng = SmallRng::from_entropy();

        #[derive(Insertable)]
        #[table_name = "thumbs"]
        struct Thumb<'a> {
            id: Id30,
            media_type: &'a str,
            data: &'a [u8],
        }

        let thumbs_id = Id30::new_random(&mut rng);

        diesel::insert_into(thumbs::table)
            .values(&Thumb {
                id: thumbs_id,
                media_type: "image/jpeg",
                data: &small_jpeg,
            })
            .execute(&*db_connection)?;

        #[derive(Insertable)]
        #[table_name = "pixurs"]
        struct Pixur {
            id: Id30,
            average_color: i32,
            thumbs_id: Id30,
        }

        let pixurs_id = Id30::new_random(&mut rng);

        diesel::insert_into(pixurs::table)
            .values(&Pixur {
                id: pixurs_id,
                average_color: ((col.channels()[0] as i32) << 16)
                    + ((col.channels()[1] as i32) << 8)
                    + ((col.channels()[2] as i32) << 0),
                thumbs_id,
            })
            .execute(&*db_connection)?;

        #[derive(Insertable)]
        #[table_name = "images"]
        struct Image<'a> {
            id: Id30,
            media_type: &'a str,
            data: &'a [u8],
        }

        let images_id = Id30::new_random(&mut rng);

        diesel::insert_into(images::table)
            .values(&Image {
                id: images_id,
                media_type: "image/jpeg",
                data: &large_jpeg,
            })
            .execute(&*db_connection)?;

        #[derive(Insertable)]
        #[table_name = "images_meta"]
        struct ImageMeta {
            id: Id30,
            width: i32,
            height: i32,
            pixurs_id: Id30,
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
