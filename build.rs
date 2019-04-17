#[macro_use]
extern crate quote;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

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

    (l * 255.).round() as u8
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("srgb.rs");
    let mut f = File::create(&dest_path).unwrap();

    let srgb_to_linear_vec: Vec<_> = (0..=255).map(srgb_to_linear).collect();
    let linear_to_srgb_vec: Vec<_> = (0..4096)
        .map(|x| linear_to_srgb((x as f32 + 0.5) / 4096.))
        .collect();

    write!(
        f,
        "{}",
        quote! {
            #[allow(unused)]
            const SRGB_TO_LINEAR: [f32; 256] = [#(#srgb_to_linear_vec),*];

            #[allow(unused)]
            const LINEAR_TO_SRGB: [u8; 4096] = [#(#linear_to_srgb_vec),*];
        }
    )
    .unwrap();
}
