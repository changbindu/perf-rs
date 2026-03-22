/// Unit tests for PerfDataReader
///
/// Tests the core reader infrastructure for parsing perf.data files.
use perf_rs::core::perf_data::{
    PerfDataReader, PERF_ATTR_SIZE_VER8, PERF_FILE_HEADER_SIZE, PERF_FILE_MAGIC,
};
use std::io::Cursor;

#[test]
fn test_parse_header_from_reference_file() {
    let path = ".sisyphus/evidence/reference-simple.perf.data";

    let reader = PerfDataReader::from_path(path).expect("Failed to open reference file");

    let header = reader.header();

    assert_eq!(header.magic, PERF_FILE_MAGIC);
    assert_eq!(header.size, PERF_FILE_HEADER_SIZE);

    assert!(header.attrs.offset > 0);
    assert!(header.attrs.size > 0);
    assert!(header.data.offset > 0);
    assert!(header.data.size > 0);
}

#[test]
fn test_parse_attributes_from_reference_file() {
    let path = ".sisyphus/evidence/reference-simple.perf.data";

    let reader = PerfDataReader::from_path(path).expect("Failed to open reference file");

    let attrs = reader.attrs();
    let event_ids = reader.event_ids();

    assert!(!attrs.is_empty(), "Should have at least one attribute");
    assert_eq!(
        attrs.len(),
        event_ids.len(),
        "Should have one ID vector per attribute"
    );

    for attr in attrs {
        assert_eq!(attr.size, PERF_ATTR_SIZE_VER8);
    }
}

#[test]
fn test_invalid_magic_number() {
    let invalid_data = vec![0u8; 104];

    let mut cursor = Cursor::new(invalid_data);

    let result = PerfDataReader::from_reader(&mut cursor);
    assert!(result.is_err(), "Should fail with invalid magic number");

    if let Err(e) = result {
        println!("Error: {:?}", e);
    }
}

#[test]
fn test_missing_file() {
    let path = ".sisyphus/evidence/nonexistent.perf.data";

    let result = PerfDataReader::from_path(path);
    assert!(result.is_err(), "Should fail for missing file");
}

#[test]
fn test_empty_file() {
    let path = ".sisyphus/evidence/reference-empty.perf.data";

    let reader = PerfDataReader::from_path(path).expect("Failed to open empty reference file");

    let header = reader.header();

    assert_eq!(header.magic, PERF_FILE_MAGIC);
    assert_eq!(header.size, PERF_FILE_HEADER_SIZE);
}

#[test]
fn test_data_section_info() {
    let path = ".sisyphus/evidence/reference-simple.perf.data";

    let reader = PerfDataReader::from_path(path).expect("Failed to open reference file");

    let data_offset = reader.data_offset();
    let data_size = reader.data_size();

    assert!(data_offset > 0, "Data offset should be positive");
    assert!(data_size > 0, "Data size should be positive");
    assert!(data_size < 1_000_000, "Data size seems too large");
}

#[test]
fn test_reader_from_cursor() {
    use std::io::Cursor;

    let path = ".sisyphus/evidence/reference-simple.perf.data";

    let data = std::fs::read(path).expect("Failed to read file");

    let mut cursor = Cursor::new(data);

    let reader =
        PerfDataReader::from_reader(&mut cursor).expect("Failed to create reader from cursor");

    let header = reader.header();

    assert_eq!(header.magic, PERF_FILE_MAGIC);
}

#[test]
fn test_multithread_reference_file() {
    let path = ".sisyphus/evidence/reference-multithread.perf.data";

    let reader =
        PerfDataReader::from_path(path).expect("Failed to open multithread reference file");

    let header = reader.header();

    assert_eq!(header.magic, PERF_FILE_MAGIC);
    assert_eq!(header.size, PERF_FILE_HEADER_SIZE);

    let attrs = reader.attrs();
    assert!(!attrs.is_empty(), "Multithread file should have attributes");

    let data_size = reader.data_size();
    assert!(
        data_size > 10000,
        "Multithread file should have substantial data"
    );
}
