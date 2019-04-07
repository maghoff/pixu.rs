use std::path::{Path, PathBuf};

use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use stopwatch::Stopwatch;
use structopt::StructOpt;

type RgbImageF32 = ImageBuffer<Rgb<f32>, Vec<f32>>;

include!(concat!(env!("OUT_DIR"), "/srgb.rs"));

fn srgb_to_linear(s: u8) -> f32 {
    SRGB_TO_LINEAR[s as usize]
}

#[cfg(none)]
fn linear_to_srgb_binary_search(l: f32) -> u8 {
    match SRGB_TO_LINEAR.binary_search_by(|x| x.partial_cmp(&l).unwrap()) {
        Ok(i) => i as u8,
        Err(i) => i as u8, // Not exactly right
    }
}

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
    linear_to_srgb_calculate(l)
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

    // let data: Vec<_> = data.into_par_iter().map(|x| srgb_to_linear(x)).collect();
    let data: Vec<_> = data.into_iter().map(|x| srgb_to_linear(x)).collect();

    RgbImageF32::from_raw(width, height, data).unwrap()
}

fn image_linear_to_srgb(src: RgbImageF32) -> RgbImage {
    let (width, height) = src.dimensions();
    let data = src.into_raw();

    // let data: Vec<_> = data.into_par_iter().map(|x| linear_to_srgb(x)).collect();
    let data: Vec<_> = data.into_iter().map(|x| linear_to_srgb(x)).collect();

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

fn ingest(file: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
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

    let (r1, r2) = rayon::join(
        || -> Result<(), std::io::Error> {
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

            Ok(())
        },
        || -> Result<(), std::io::Error> {
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

            let (r1, _) = rayon::join(
                || -> Result<(), std::io::Error> {
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

                    Ok(())
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
                },
            );

            r1?;

            Ok(())
        },
    );

    r1?;
    r2?;

    Ok(())
}

#[derive(StructOpt)]
struct Params {
    files: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Params::from_args();

    let mut ok = true;

    for file in args.files {
        let sw = Stopwatch::start_new();
        match ingest(&file) {
            Ok(()) => eprintln!("Ingested {} in {}ms", file.display(), sw.elapsed_ms()),
            Err(err) => {
                ok = false;
                eprintln!("Error ingesting {}: {}", file.display(), err);
            }
        }
    }

    std::process::exit(if ok { 0 } else { 1 });
}
