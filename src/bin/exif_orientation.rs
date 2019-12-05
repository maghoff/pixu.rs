fn main() {
    let file = std::fs::File::open(std::env::args().skip(1).next().unwrap()).unwrap();

    let orientation = exif::Reader::new(&mut std::io::BufReader::new(&file))
        .ok()
        .as_ref()
        .and_then(|reader| reader.get_field(exif::Tag::Orientation, false))
        .and_then(|x| x.value.get_uint(0))
        .unwrap_or(1)
        - 1;

    eprintln!("ORG: Orientation: {:03b}", orientation);
}
