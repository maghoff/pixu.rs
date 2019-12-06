use std::convert::TryInto;

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

    eprintln!("Transformation matrix: {:?}", t);

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

fn encode_jpeg(img: image::RgbImage, quality: u8) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut encoder = image::jpeg::JPEGEncoder::new_with_quality(&mut buf, quality);
    let (width, height) = (img.width(), img.height());
    encoder.encode(&img.into_raw(), width, height, image::ColorType::RGB(8))?;
    Ok(buf)
}

fn main() {
    let filename = std::env::args().skip(1).next().unwrap();

    let file = std::fs::File::open(&filename).unwrap();
    let img = image::load(
        &mut std::io::BufReader::new(&file),
        image::ImageFormat::JPEG,
    )
    .unwrap()
    .to_rgb();
    eprintln!("ORG: {}x{}", img.width(), img.height());

    let file = std::fs::File::open(&filename).unwrap();
    let orientation = exif::Reader::new(&mut std::io::BufReader::new(&file))
        .ok()
        .as_ref()
        .and_then(|reader| reader.get_field(exif::Tag::Orientation, false))
        .and_then(|x| x.value.get_uint(0))
        .unwrap_or(1)
        - 1;

    eprintln!("ORG: Orientation: {:03b}", orientation);

    let new_img = transform_by_orientation(img, orientation);

    std::fs::write("output.jpg", &encode_jpeg(new_img, 80).unwrap()).unwrap();
}
