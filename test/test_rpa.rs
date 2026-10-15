use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom, Write};
use flate2::write::ZlibEncoder;
use flate2::Compression;

use rpa::models::version::RpaVersion;
use rpa::rpa::parser::RpaParser;
use rpa::rpa::reader::RpaReader;

#[test]
fn test_version_header_parsing() {
    // RPA-2.0
    let (v, offset, key) = RpaVersion::parse_header("RPA-2.0 0000000000232d24\n").unwrap();
    assert_eq!(v, RpaVersion::Rpa2);
    assert_eq!(offset, 0x232D24);
    assert_eq!(key, 0);

    // RPA-3.0 with single key
    let (v, offset, key) = RpaVersion::parse_header("RPA-3.0 0000000000001000 12345678\n").unwrap();
    assert_eq!(v, RpaVersion::Rpa3);
    assert_eq!(offset, 0x1000);
    assert_eq!(key, 0x12345678);

    // RPA-3.0 with multiple XOR keys
    let (v, offset, key) =
        RpaVersion::parse_header("RPA-3.0 0000000000001000 deadbeef 12345678\n").unwrap();
    assert_eq!(v, RpaVersion::Rpa3);
    assert_eq!(offset, 0x1000);
    assert_eq!(key, 0xDEADBEEF ^ 0x12345678);

    // RPA-3.2
    let (v, offset, key) =
        RpaVersion::parse_header("RPA-3.2 0000000000002000 dummyval aabbccdd 11223344\n").unwrap();
    assert_eq!(v, RpaVersion::Rpa3_2);
    assert_eq!(offset, 0x2000);
    assert_eq!(key, 0xAABBCCDD ^ 0x11223344);
}

#[test]
fn test_parse_real_sample_rpa() {
    let path = "sample.rpa";
    if !std::path::Path::new(path).exists() {
        println!("sample.rpa not found, skipping");
        return;
    }

    let mut file = File::open(path).expect("Failed to open sample.rpa");
    let archive = RpaParser::parse(&mut file).expect("Failed to parse sample.rpa");

    assert_eq!(archive.version, RpaVersion::Rpa2);
    assert_eq!(archive.len(), 67);

    // Verify some specific entries
    let font_entry = archive
        .find_by_path("0x52-URM/Caveat.ttf")
        .expect("Caveat.ttf should exist");
    assert_eq!(font_entry.offset(), 86287);
    assert_eq!(font_entry.length(), 391068);

    // Read font bytes and verify TTF magic header
    let font_bytes = RpaReader::read_entry(&mut file, font_entry).expect("Read font");
    assert_eq!(font_bytes.len(), 391068);
    // TrueType font magic: 0x00, 0x01, 0x00, 0x00
    assert_eq!(&font_bytes[0..4], &[0x00, 0x01, 0x00, 0x00]);

    // Verify JPEG banner
    let jpg_entry = archive
        .find_by_path("0x52-URM/docs/Banner_URM.jpg")
        .expect("Banner_URM.jpg should exist");
    let jpg_bytes = RpaReader::read_entry(&mut file, jpg_entry).expect("Read jpg");
    assert_eq!(jpg_bytes.len(), 80277);
    // JPEG magic: FF D8 FF
    assert_eq!(&jpg_bytes[0..3], &[0xFF, 0xD8, 0xFF]);
}

#[test]
fn test_synthetic_rpa3_with_obfuscation_and_prefix() {
    // Construct a synthetic in-memory RPA-3.0 file:
    // Header (34 bytes): RPA-3.0 <16-hex-offset> <8-hex-key>\n
    let key: u32 = 0x42424242;
    let file1_payload = b"Hello, Ren'Py RPA World!";
    let file2_prefix = b"RENPY_SECRET_PREFIX_";
    let file2_body = b"Additional payload data goes here.";
    let file2_full_len = (file2_prefix.len() + file2_body.len()) as u64;

    let mut cursor = Cursor::new(Vec::new());

    // Reserve 34 bytes for header
    let header_placeholder = vec![0u8; 34];
    cursor.write_all(&header_placeholder).unwrap();

    // Write file1
    let f1_offset = cursor.position();
    cursor.write_all(file1_payload).unwrap();
    let f1_len = file1_payload.len() as u64;

    // Write file2 (body only, prefix is stored in index)
    let f2_offset = cursor.position();
    cursor.write_all(file2_body).unwrap();

    // Now write index at current position
    let index_offset = cursor.position();

    // Build pickle dict:
    // { "script/hello.txt": [(f1_offset ^ key, f1_len ^ key)],
    //   "images/bg.png": [(f2_offset ^ key, file2_full_len ^ key, prefix)] }
    use std::collections::BTreeMap;

    #[derive(serde::Serialize)]
    #[serde(untagged)]
    enum Entry {
        Two(u64, u64),
        Three(u64, u64, Vec<u8>),
    }

    let mut dict: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    dict.insert(
        "script/hello.txt".into(),
        vec![Entry::Two(f1_offset ^ (key as u64), f1_len ^ (key as u64))],
    );
    dict.insert(
        "images/bg.png".into(),
        vec![Entry::Three(
            f2_offset ^ (key as u64),
            file2_full_len ^ (key as u64),
            file2_prefix.to_vec(),
        )],
    );

    let pickle_data = serde_pickle::to_vec(&dict, serde_pickle::SerOptions::new()).unwrap();

    // Compress pickle with zlib
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&pickle_data).unwrap();
    let compressed_index = encoder.finish().unwrap();

    cursor.write_all(&compressed_index).unwrap();

    // Go back and write real header
    cursor.seek(SeekFrom::Start(0)).unwrap();
    let header_str = format!("RPA-3.0 {:016x} {:08x}\n", index_offset, key);
    assert_eq!(header_str.len(), 34);
    cursor.write_all(header_str.as_bytes()).unwrap();

    // Now test parsing with RpaParser
    let archive = RpaParser::parse(&mut cursor).expect("Parse synthetic RPA3");
    assert_eq!(archive.version, RpaVersion::Rpa3);
    assert_eq!(archive.key, key);
    assert_eq!(archive.len(), 2);

    let f1 = archive.find_by_path("script/hello.txt").unwrap();
    assert_eq!(f1.offset(), f1_offset);
    assert_eq!(f1.length(), f1_len);
    let f1_read = RpaReader::read_entry(&mut cursor, f1).unwrap();
    assert_eq!(f1_read, file1_payload);

    let f2 = archive.find_by_path("images/bg.png").unwrap();
    assert_eq!(f2.offset(), f2_offset);
    assert_eq!(f2.length(), file2_full_len);
    let f2_read = RpaReader::read_entry(&mut cursor, f2).unwrap();
    let mut expected_f2 = Vec::new();
    expected_f2.extend_from_slice(file2_prefix);
    expected_f2.extend_from_slice(file2_body);
    assert_eq!(f2_read, expected_f2);
}
