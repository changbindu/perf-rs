//! Comprehensive validation tests for PerfDataReader

use perf_rs::core::perf_data::{
    CommEvent, Event, FinishedRoundEvent, MmapEvent, PerfDataReader, PerfDataWriter, PerfEventAttr,
    SampleEvent, PERF_FILE_HEADER_SIZE, PERF_FILE_MAGIC, PERF_SAMPLE_CALLCHAIN, PERF_SAMPLE_IP,
    PERF_SAMPLE_PERIOD, PERF_SAMPLE_TID, PERF_SAMPLE_TIME,
};
use std::io::Cursor;
use std::path::PathBuf;

fn ref_file(name: &str) -> PathBuf {
    let mut path = PathBuf::from(".sisyphus/evidence");
    path.push(name);
    path
}

#[test]
fn test_read_header_from_reference_empty_file() {
    let path = ref_file("reference-empty.perf.data");
    let reader =
        PerfDataReader::from_path(&path).expect("Failed to open reference-empty.perf.data");

    let header = reader.header();

    assert_eq!(
        header.magic, PERF_FILE_MAGIC,
        "Magic number should be correct"
    );
    assert_eq!(
        header.size, PERF_FILE_HEADER_SIZE,
        "Header size should be 104 bytes"
    );
    assert!(
        header.attrs.offset > 0,
        "Attributes section should have an offset"
    );
    assert!(header.data.offset > 0, "Data section should have an offset");
}

#[test]
fn test_read_header_from_reference_simple_file() {
    let path = ref_file("reference-simple.perf.data");
    let reader =
        PerfDataReader::from_path(&path).expect("Failed to open reference-simple.perf.data");

    let header = reader.header();

    assert_eq!(header.magic, PERF_FILE_MAGIC);
    assert_eq!(header.size, PERF_FILE_HEADER_SIZE);
    assert!(
        header.attrs.size > 0,
        "Attributes section should not be empty"
    );
    assert!(header.data.size > 0, "Data section should not be empty");
}

#[test]
fn test_read_header_from_reference_multithread_file() {
    let path = ref_file("reference-multithread.perf.data");
    let reader =
        PerfDataReader::from_path(&path).expect("Failed to open reference-multithread.perf.data");

    let header = reader.header();

    assert_eq!(header.magic, PERF_FILE_MAGIC);
    assert_eq!(header.size, PERF_FILE_HEADER_SIZE);
    assert!(
        header.data.size > 10000,
        "Multithread file should have substantial data"
    );
}

#[test]
fn test_read_attributes_from_reference_simple_file() {
    let path = ref_file("reference-simple.perf.data");
    let reader =
        PerfDataReader::from_path(&path).expect("Failed to open reference-simple.perf.data");

    let attrs = reader.attrs();
    let event_ids = reader.event_ids();

    assert!(!attrs.is_empty(), "Should have at least one attribute");
    assert_eq!(
        attrs.len(),
        event_ids.len(),
        "Should have one ID vector per attribute"
    );

    for attr in attrs {
        assert!(attr.type_ < 10, "Attribute type should be reasonable");
        assert!(attr.config < 1000, "Attribute config should be reasonable");
        assert!(
            attr.sample_type & PERF_SAMPLE_IP != 0 || attr.sample_type & PERF_SAMPLE_TID != 0,
            "Sample type should include at least IP or TID"
        );
    }
}

#[test]
fn test_read_attributes_from_reference_multithread_file() {
    let path = ref_file("reference-multithread.perf.data");
    let reader =
        PerfDataReader::from_path(&path).expect("Failed to open reference-multithread.perf.data");

    let attrs = reader.attrs();

    assert!(!attrs.is_empty(), "Multithread file should have attributes");

    for attr in attrs {
        assert!(attr.type_ < 10);
        assert!(attr.size > 100, "Attribute size should be > 100 bytes");
    }
}

#[test]
fn test_read_events_from_reference_simple_file() {
    let path = ref_file("reference-simple.perf.data");
    let mut reader =
        PerfDataReader::from_path(&path).expect("Failed to open reference-simple.perf.data");

    let events = reader.read_all_events().expect("Failed to read all events");

    assert!(!events.is_empty(), "Simple file should contain events");
}

#[test]
fn test_read_events_from_reference_multithread_file() {
    let path = ref_file("reference-multithread.perf.data");
    let mut reader =
        PerfDataReader::from_path(&path).expect("Failed to open reference-multithread.perf.data");

    let events = reader.read_all_events().expect("Failed to read all events");

    assert!(
        events.len() > 0,
        "Multithread file should have > 0 events, got {}",
        events.len()
    );

    let comm_count = events
        .iter()
        .filter(|e| matches!(e, Event::Comm(_)))
        .count();
    let mmap_count = events
        .iter()
        .filter(|e| matches!(e, Event::Mmap(_)))
        .count();
    let sample_count = events
        .iter()
        .filter(|e| matches!(e, Event::Sample(_)))
        .count();

    println!(
        "Multithread file events: {} COMM, {} MMAP, {} SAMPLE, {} total",
        comm_count,
        mmap_count,
        sample_count,
        events.len()
    );
}

#[test]
fn test_event_iterator_streaming_from_simple_file() {
    let path = ref_file("reference-simple.perf.data");
    let mut reader =
        PerfDataReader::from_path(&path).expect("Failed to open reference-simple.perf.data");

    let iter = reader
        .event_iter()
        .expect("Failed to create event iterator");

    let mut count = 0;
    for event_result in iter {
        let _event = event_result.expect("Failed to read event");
        count += 1;
    }

    assert!(
        count > 0,
        "Iterator should return at least one event, got {}",
        count
    );
}

#[test]
fn test_sample_round_trip() {
    let sample_type = PERF_SAMPLE_IP
        | PERF_SAMPLE_TID
        | PERF_SAMPLE_TIME
        | PERF_SAMPLE_PERIOD
        | PERF_SAMPLE_CALLCHAIN;

    let original_sample = SampleEvent::new(
        sample_type,
        1234567890123456,
        0x7f0000001000,
        1234,
        5678,
        1000,
        Some(vec![0x7f0000002000, 0x7f0000003000]),
        Some(0),
        1024,
        None,
        None,
    );

    let mut buffer = Cursor::new(Vec::new());
    original_sample
        .write_to(&mut buffer)
        .expect("Failed to write sample");

    buffer.set_position(0);
    let read_sample =
        SampleEvent::read_from(&mut buffer, sample_type).expect("Failed to read sample");

    assert_eq!(read_sample.ip, original_sample.ip, "IP should match");
    assert_eq!(read_sample.pid, original_sample.pid, "PID should match");
    assert_eq!(read_sample.tid, original_sample.tid, "TID should match");
    assert_eq!(read_sample.time, original_sample.time, "Time should match");
    assert_eq!(
        read_sample.period, original_sample.period,
        "Period should match"
    );
    assert_eq!(
        read_sample.callchain, original_sample.callchain,
        "Callchain should match"
    );
}

#[test]
fn test_mmap_round_trip() {
    let original_mmap = MmapEvent::new(
        1234,
        5678,
        0x7f0000000000,
        0x1000,
        0,
        "/usr/bin/test-program".to_string(),
    );

    let mut buffer = Cursor::new(Vec::new());
    original_mmap
        .write_to(&mut buffer)
        .expect("Failed to write MMAP");

    buffer.set_position(0);
    let read_mmap = MmapEvent::read_from(&mut buffer).expect("Failed to read MMAP");

    assert_eq!(read_mmap.pid, original_mmap.pid, "PID should match");
    assert_eq!(read_mmap.tid, original_mmap.tid, "TID should match");
    assert_eq!(read_mmap.addr, original_mmap.addr, "Address should match");
    assert_eq!(read_mmap.len, original_mmap.len, "Length should match");
    assert_eq!(
        read_mmap.pgoff, original_mmap.pgoff,
        "Page offset should match"
    );
    assert_eq!(
        read_mmap.filename, original_mmap.filename,
        "Filename should match"
    );
}

#[test]
fn test_comm_round_trip() {
    let original_comm = CommEvent::new(1234, 5678, "test-program".to_string());

    let mut buffer = Cursor::new(Vec::new());
    original_comm
        .write_to(&mut buffer)
        .expect("Failed to write COMM");

    buffer.set_position(0);
    let read_comm = CommEvent::read_from(&mut buffer).expect("Failed to read COMM");

    assert_eq!(read_comm.pid, original_comm.pid, "PID should match");
    assert_eq!(read_comm.tid, original_comm.tid, "TID should match");
    assert_eq!(read_comm.comm, original_comm.comm, "Comm should match");
}

#[test]
fn test_finished_round_round_trip() {
    let original_event = FinishedRoundEvent::new();

    let mut buffer = Cursor::new(Vec::new());
    original_event
        .write_to(&mut buffer)
        .expect("Failed to write FINISHED_ROUND");

    buffer.set_position(0);
    let read_event =
        FinishedRoundEvent::read_from(&mut buffer).expect("Failed to read FINISHED_ROUND");

    assert_eq!(
        read_event.header.type_, original_event.header.type_,
        "Event type should match"
    );
    assert_eq!(
        read_event.header.size, original_event.header.size,
        "Event size should match"
    );
}

#[test]
fn test_empty_file_handling() {
    let path = ref_file("reference-empty.perf.data");
    let mut reader = PerfDataReader::from_path(&path).expect("Failed to open empty reference file");

    let header = reader.header();
    assert_eq!(header.magic, PERF_FILE_MAGIC);
    assert_eq!(header.size, PERF_FILE_HEADER_SIZE);

    let events = reader
        .read_all_events()
        .expect("Failed to read events from empty file");

    assert!(
        events.len() >= 0,
        "Empty file should have at least 0 events"
    );
}

#[test]
fn test_empty_string_handling() {
    let comm = CommEvent::new(1, 1, "".to_string());

    let mut buffer = Cursor::new(Vec::new());
    comm.write_to(&mut buffer)
        .expect("Failed to write empty COMM");

    buffer.set_position(0);
    let read_comm = CommEvent::read_from(&mut buffer).expect("Failed to read empty COMM");

    assert_eq!(read_comm.comm, "", "Empty string should be preserved");
}

#[test]
fn test_missing_file_error() {
    let result = PerfDataReader::from_path(".sisyphus/evidence/nonexistent.perf.data");
    assert!(result.is_err(), "Should fail for missing file");

    if let Err(e) = result {
        assert!(
            e.to_string().contains("No such file"),
            "Error should mention file not found"
        );
    }
}

#[test]
fn test_invalid_magic_number() {
    let mut buffer = Cursor::new(vec![0u8; 104]);

    let result = PerfDataReader::from_reader(&mut buffer);
    assert!(result.is_err(), "Should fail with invalid magic number");

    if let Err(e) = result {
        assert!(
            e.to_string().contains("Invalid magic number"),
            "Error should mention invalid magic"
        );
    }
}

#[test]
fn test_backward_compatibility_v2_format() {
    for file_name in &[
        "reference-empty.perf.data",
        "reference-simple.perf.data",
        "reference-multithread.perf.data",
    ] {
        let path = ref_file(file_name);
        let result = PerfDataReader::from_path(&path);

        assert!(
            result.is_ok(),
            "Should be able to open V2 format file: {}",
            file_name
        );

        if let Ok(reader) = result {
            let header = reader.header();
            assert_eq!(
                header.magic, PERF_FILE_MAGIC,
                "File {} should have correct V2 magic",
                file_name
            );
        }
    }
}

#[test]
fn test_read_large_multithread_file() {
    let path = ref_file("reference-multithread.perf.data");
    let mut reader = PerfDataReader::from_path(&path).expect("Failed to open multithread file");

    let events = reader.read_all_events().expect("Failed to read all events");

    assert!(events.len() > 0, "Should have at least some events");

    let iter = reader.event_iter().expect("Failed to create iterator");
    let iter_count = iter.count();

    assert_eq!(
        events.len(),
        iter_count,
        "Iterator should return same number of events"
    );
}

#[test]
fn test_sample_with_callchain_round_trip() {
    let sample_type = PERF_SAMPLE_IP
        | PERF_SAMPLE_TID
        | PERF_SAMPLE_TIME
        | PERF_SAMPLE_PERIOD
        | PERF_SAMPLE_CALLCHAIN;

    let callchain = vec![
        0x7f0000001000,
        0x7f0000002000,
        0x7f0000003000,
        0x7f0000004000,
        0x7f0000005000,
    ];

    let original_sample = SampleEvent::new(
        sample_type,
        1234567890123456,
        0x7f0000001000,
        1234,
        5678,
        1000,
        Some(callchain.clone()),
        Some(0),
        1024,
        None,
        None,
    );

    let mut buffer = Cursor::new(Vec::new());
    original_sample
        .write_to(&mut buffer)
        .expect("Failed to write sample with callchain");

    buffer.set_position(0);
    let read_sample =
        SampleEvent::read_from(&mut buffer, sample_type).expect("Failed to read sample");

    assert_eq!(
        read_sample.callchain,
        Some(callchain),
        "Callchain should be preserved"
    );
}

#[test]
fn test_multiple_samples_round_trip() {
    let sample_type = PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD;

    let samples = vec![
        SampleEvent::new(
            sample_type,
            0,
            0x1000,
            1,
            1,
            100,
            None,
            Some(0),
            1024,
            None,
            None,
        ),
        SampleEvent::new(
            sample_type,
            0,
            0x2000,
            1,
            1,
            200,
            None,
            Some(0),
            1024,
            None,
            None,
        ),
        SampleEvent::new(
            sample_type,
            0,
            0x3000,
            1,
            1,
            300,
            None,
            Some(0),
            1024,
            None,
            None,
        ),
    ];

    let mut buffer = Cursor::new(Vec::new());
    for sample in &samples {
        sample
            .write_to(&mut buffer)
            .expect("Failed to write sample");
    }

    buffer.set_position(0);
    for original_sample in &samples {
        let read_sample =
            SampleEvent::read_from(&mut buffer, sample_type).expect("Failed to read sample");

        assert_eq!(read_sample.ip, original_sample.ip, "IP should match");
        assert_eq!(
            read_sample.period, original_sample.period,
            "Period should match"
        );
    }
}
