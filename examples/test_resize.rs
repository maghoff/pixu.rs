use image::{ImageBuffer, Pixel};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = image::load_from_memory(include_bytes!("alessipus.jpg"))?.to_rgb();

    let mut img = ImageBuffer::new(src.width(), src.height());
    for (to, from) in img.pixels_mut().zip(src.pixels()) {
        let ch = from.channels();
        *to = image::Rgb::from_channels(
            srgb_to_linear(ch[0]),
            srgb_to_linear(ch[1]),
            srgb_to_linear(ch[2]),
            1.,
        )
    }

    let smol = image::imageops::resize(&img, 320, 213, image::imageops::Lanczos3);

    let mut smol2 = ImageBuffer::new(smol.width(), smol.height());
    for (to, from) in smol2.pixels_mut().zip(smol.pixels()) {
        let ch = from.channels();
        *to = image::Rgb::from_channels(
            linear_to_srgb(ch[0]),
            linear_to_srgb(ch[1]),
            linear_to_srgb(ch[2]),
            1,
        )
    }

    smol2.save("smol.jpg")?;

    Ok(())
}
