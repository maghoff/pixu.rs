use std::path::{Path, PathBuf};

use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use structopt::StructOpt;

type RgbImageF32 = ImageBuffer<Rgb<f32>, Vec<f32>>;

fn srgb_to_linear(s: u8) -> f32 {
    match s as f32 / 255. {
        s if s < 0.04045 => s / 12.92,
        s => ((s + 0.055) / 1.055).powf(2.4),
    }
}

fn linear_to_srgb(l: f32) -> u8 {
    let l = match l {
        l if l < 0. => 0.,
        l if l > 1. => 1.,
        l if l < 0.0031308 => (l * 12.92),
        l => 1.055 * l.powf(1.0 / 2.4) - 0.055,
    };

    (l * 255.) as u8
}

fn px_srgb_to_linear(s: &Rgb<u8>) -> Rgb<f32> {
    let ch = s.channels();
    Rgb::from_channels(
        srgb_to_linear(ch[0]),
        srgb_to_linear(ch[1]),
        srgb_to_linear(ch[2]),
        0.,
    )
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

fn image_srgb_to_linear(src: &RgbImage) -> RgbImageF32 {
    let mut img = ImageBuffer::new(src.width(), src.height());

    for (to, from) in img.pixels_mut().zip(src.pixels()) {
        *to = px_srgb_to_linear(from);
    }

    img
}

fn image_linear_to_srgb(src: &RgbImageF32) -> RgbImage {
    let mut img = ImageBuffer::new(src.width(), src.height());

    for (to, from) in img.pixels_mut().zip(src.pixels()) {
        *to = px_linear_to_srgb(from);
    }

    img
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

fn ingest(file: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(file.as_ref())?.to_rgb();
    let img = image_srgb_to_linear(&img);

    let large = if img.width() > 2560 {
        let nwidth = 2560;
        let nheight = nwidth * img.height() / img.width();
        image::imageops::resize(&img, nwidth, nheight, image::imageops::Lanczos3)
    } else {
        img
    };

    let _large_srgb = image_linear_to_srgb(&large);
    // Store _large_srgb

    let nwidth = 320;
    let nheight = nwidth * large.height() / large.width();
    let small = image::imageops::resize(&large, nwidth, nheight, image::imageops::Lanczos3);
    let _small_srgb = image_linear_to_srgb(&small);
    // Store _small_srgb

    let _col = avg_color(&small);
    // Store _col

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
        let res = ingest(&file);

        if let Err(err) = res {
            ok = false;
            eprintln!("Error ingesting {}: {}", file.display(), err);
        }
    }

    std::process::exit(if ok { 0 } else { 1 });
}
