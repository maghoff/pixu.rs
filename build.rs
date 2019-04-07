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

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("srgb.rs");
    let mut f = File::create(&dest_path).unwrap();

    let srgb_to_linear_vec: Vec<_> = (0..=255).map(srgb_to_linear).collect();

    write!(
        f,
        "{}",
        quote! {
            const SRGB_TO_LINEAR: [f32; 256] = [#(#srgb_to_linear_vec),*];
        }
    )
    .unwrap();
}
