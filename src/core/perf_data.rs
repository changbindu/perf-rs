//! perf.data file format support for reading and writing profiling data.
//!
//! This module implements a simplified custom format for MVP that supports
//! basic event types needed for record/report functionality.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::error::PerfError;

/// Magic bytes identifying perf-rs format (PERFRS01)
const MAGIC: &[u8; 8] = b"PERFRS01";

/// Current format version
const VERSION: u32 = 1;

/// Header size in bytes
const HEADER_SIZE: u32 = 64;

/// Event header size in bytes
const EVENT_HEADER_SIZE: u16 = 24;

/// Event type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum EventType {
    Mmap = 1,
    Comm = 2,
    Sample = 3,
}

impl TryFrom<u16> for EventType {
    type Error = PerfError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(EventType::Mmap),
            2 => Ok(EventType::Comm),
            3 => Ok(EventType::Sample),
            _ => Err(PerfError::InvalidEventType { event_type: value }),
        }
    }
}

/// File header structure (64 bytes)
#[derive(Debug, Clone)]
pub struct Header {
    /// Magic bytes: "PERFRS01"
    pub magic: [u8; 8],
    /// Format version
    pub version: u32,
    /// Header size in bytes
    pub header_size: u32,
    /// Total sample event count
    pub sample_count: u64,
    /// Total mmap event count
    pub mmap_count: u64,
    /// Total comm event count
    pub comm_count: u64,
    /// Reserved space
    pub reserved: [u8; 32],
}

impl Default for Header {
    fn default() -> Self {
        Self {
            magic: *MAGIC,
            version: VERSION,
            header_size: HEADER_SIZE,
            sample_count: 0,
            mmap_count: 0,
            comm_count: 0,
            reserved: [0u8; 32],
        }
    }
}

impl Header {
    /// Write header to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.magic)?;
        writer.write_all(&self.version.to_le_bytes())?;
        writer.write_all(&self.header_size.to_le_bytes())?;
        writer.write_all(&self.sample_count.to_le_bytes())?;
        writer.write_all(&self.mmap_count.to_le_bytes())?;
        writer.write_all(&self.comm_count.to_le_bytes())?;
        writer.write_all(&self.reserved)?;
        Ok(())
    }

    /// Read header from a reader
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;

        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let version = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let header_size = u32::from_le_bytes(buf);

        let mut buf8 = [0u8; 8];
        reader.read_exact(&mut buf8)?;
        let sample_count = u64::from_le_bytes(buf8);

        reader.read_exact(&mut buf8)?;
        let mmap_count = u64::from_le_bytes(buf8);

        reader.read_exact(&mut buf8)?;
        let comm_count = u64::from_le_bytes(buf8);

        let mut reserved = [0u8; 32];
        reader.read_exact(&mut reserved)?;

        Ok(Self {
            magic,
            version,
            header_size,
            sample_count,
            mmap_count,
            comm_count,
            reserved,
        })
    }

    /// Validate header magic and version
    pub fn validate(&self) -> Result<(), PerfError> {
        if &self.magic != MAGIC {
            return Err(PerfError::InvalidMagic {
                expected: String::from_utf8_lossy(MAGIC).to_string(),
                actual: String::from_utf8_lossy(&self.magic).to_string(),
            });
        }
        if self.version != VERSION {
            return Err(PerfError::UnsupportedVersion {
                version: self.version,
            });
        }
        Ok(())
    }
}

/// Event header structure (24 bytes)
#[derive(Debug, Clone)]
pub struct EventHeader {
    /// Event type (MMAP, COMM, SAMPLE)
    pub event_type: EventType,
    /// Total event size including header
    pub size: u16,
    /// Timestamp
    pub time: u64,
    /// Reserved space
    pub reserved: u32,
}

impl EventHeader {
    /// Create new event header
    pub fn new(event_type: EventType, size: u16, time: u64) -> Self {
        Self {
            event_type,
            size,
            time,
            reserved: 0,
        }
    }

    /// Write event header to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&(self.event_type as u16).to_le_bytes())?;
        writer.write_all(&self.size.to_le_bytes())?;
        writer.write_all(&self.time.to_le_bytes())?;
        writer.write_all(&self.reserved.to_le_bytes())?;
        Ok(())
    }

    /// Read event header from a reader
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, PerfError> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        let event_type_u16 = u16::from_le_bytes(buf);
        let event_type = EventType::try_from(event_type_u16)?;

        reader.read_exact(&mut buf)?;
        let size = u16::from_le_bytes(buf);

        let mut buf8 = [0u8; 8];
        reader.read_exact(&mut buf8)?;
        let time = u64::from_le_bytes(buf8);

        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        let reserved = u32::from_le_bytes(buf4);

        Ok(Self {
            event_type,
            size,
            time,
            reserved,
        })
    }
}

/// Sample event data
#[derive(Debug, Clone)]
pub struct SampleEvent {
    /// Event header
    pub header: EventHeader,
    /// Instruction pointer
    pub ip: u64,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
    /// Sampling period
    pub period: u64,
    /// Callchain (stack trace)
    pub callchain: Vec<u64>,
}

impl SampleEvent {
    /// Create new sample event
    pub fn new(time: u64, ip: u64, pid: u32, tid: u32, period: u64, callchain: Vec<u64>) -> Self {
        let callchain_len = callchain.len() as u32;
        let size = EVENT_HEADER_SIZE + 8 + 4 + 4 + 8 + 4 + (callchain_len as u16 * 8);

        let header = EventHeader::new(EventType::Sample, size, time);

        Self {
            header,
            ip,
            pid,
            tid,
            period,
            callchain,
        }
    }

    /// Write sample event to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.header.write_to(writer)?;
        writer.write_all(&self.ip.to_le_bytes())?;
        writer.write_all(&self.pid.to_le_bytes())?;
        writer.write_all(&self.tid.to_le_bytes())?;
        writer.write_all(&self.period.to_le_bytes())?;
        writer.write_all(&(self.callchain.len() as u32).to_le_bytes())?;
        for addr in &self.callchain {
            writer.write_all(&addr.to_le_bytes())?;
        }
        Ok(())
    }

    /// Read sample event from a reader
    pub fn read_from<R: Read>(header: EventHeader, reader: &mut R) -> io::Result<Self> {
        let mut buf8 = [0u8; 8];
        reader.read_exact(&mut buf8)?;
        let ip = u64::from_le_bytes(buf8);

        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        let pid = u32::from_le_bytes(buf4);

        reader.read_exact(&mut buf4)?;
        let tid = u32::from_le_bytes(buf4);

        reader.read_exact(&mut buf8)?;
        let period = u64::from_le_bytes(buf8);

        reader.read_exact(&mut buf4)?;
        let callchain_len = u32::from_le_bytes(buf4);

        let mut callchain = Vec::with_capacity(callchain_len as usize);
        for _ in 0..callchain_len {
            reader.read_exact(&mut buf8)?;
            callchain.push(u64::from_le_bytes(buf8));
        }

        Ok(Self {
            header,
            ip,
            pid,
            tid,
            period,
            callchain,
        })
    }
}

/// MMAP event data (memory mapping)
#[derive(Debug, Clone)]
pub struct MmapEvent {
    /// Event header
    pub header: EventHeader,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
    /// Mapping start address
    pub addr: u64,
    /// Mapping length
    pub len: u64,
    /// Page offset
    pub pgoff: u64,
    /// File name (null-terminated)
    pub filename: String,
}

impl MmapEvent {
    /// Create new mmap event
    pub fn new(
        time: u64,
        pid: u32,
        tid: u32,
        addr: u64,
        len: u64,
        pgoff: u64,
        filename: String,
    ) -> Self {
        let filename_bytes = filename.as_bytes();
        let filename_len = filename_bytes.len() + 1; // +1 for null terminator
        let size = EVENT_HEADER_SIZE + 4 + 4 + 8 + 8 + 8 + (filename_len as u16);

        let header = EventHeader::new(EventType::Mmap, size, time);

        Self {
            header,
            pid,
            tid,
            addr,
            len,
            pgoff,
            filename,
        }
    }

    /// Write mmap event to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.header.write_to(writer)?;
        writer.write_all(&self.pid.to_le_bytes())?;
        writer.write_all(&self.tid.to_le_bytes())?;
        writer.write_all(&self.addr.to_le_bytes())?;
        writer.write_all(&self.len.to_le_bytes())?;
        writer.write_all(&self.pgoff.to_le_bytes())?;
        writer.write_all(self.filename.as_bytes())?;
        writer.write_all(&[0])?; // null terminator
        Ok(())
    }

    /// Read mmap event from a reader
    pub fn read_from<R: Read>(header: EventHeader, reader: &mut R) -> io::Result<Self> {
        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        let pid = u32::from_le_bytes(buf4);

        reader.read_exact(&mut buf4)?;
        let tid = u32::from_le_bytes(buf4);

        let mut buf8 = [0u8; 8];
        reader.read_exact(&mut buf8)?;
        let addr = u64::from_le_bytes(buf8);

        reader.read_exact(&mut buf8)?;
        let len = u64::from_le_bytes(buf8);

        reader.read_exact(&mut buf8)?;
        let pgoff = u64::from_le_bytes(buf8);

        // Read filename until null terminator
        let data_size = header.size as usize - EVENT_HEADER_SIZE as usize - 4 - 4 - 8 - 8 - 8;
        let mut filename_bytes = vec![0u8; data_size];
        reader.read_exact(&mut filename_bytes)?;

        // Remove null terminator and convert to string
        let filename = String::from_utf8_lossy(
            &filename_bytes[..filename_bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(data_size)],
        )
        .into_owned();

        Ok(Self {
            header,
            pid,
            tid,
            addr,
            len,
            pgoff,
            filename,
        })
    }
}

/// COMM event data (process name change)
#[derive(Debug, Clone)]
pub struct CommEvent {
    /// Event header
    pub header: EventHeader,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
    /// Command name (null-terminated)
    pub comm: String,
}

impl CommEvent {
    /// Create new comm event
    pub fn new(time: u64, pid: u32, tid: u32, comm: String) -> Self {
        let comm_bytes = comm.as_bytes();
        let comm_len = comm_bytes.len() + 1; // +1 for null terminator
        let size = EVENT_HEADER_SIZE + 4 + 4 + (comm_len as u16);

        let header = EventHeader::new(EventType::Comm, size, time);

        Self {
            header,
            pid,
            tid,
            comm,
        }
    }

    /// Write comm event to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.header.write_to(writer)?;
        writer.write_all(&self.pid.to_le_bytes())?;
        writer.write_all(&self.tid.to_le_bytes())?;
        writer.write_all(self.comm.as_bytes())?;
        writer.write_all(&[0])?; // null terminator
        Ok(())
    }

    /// Read comm event from a reader
    pub fn read_from<R: Read>(header: EventHeader, reader: &mut R) -> io::Result<Self> {
        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        let pid = u32::from_le_bytes(buf4);

        reader.read_exact(&mut buf4)?;
        let tid = u32::from_le_bytes(buf4);

        // Read comm until null terminator
        let data_size = header.size as usize - EVENT_HEADER_SIZE as usize - 4 - 4;
        let mut comm_bytes = vec![0u8; data_size];
        reader.read_exact(&mut comm_bytes)?;

        // Remove null terminator and convert to string
        let comm = String::from_utf8_lossy(
            &comm_bytes[..comm_bytes.iter().position(|&b| b == 0).unwrap_or(data_size)],
        )
        .into_owned();

        Ok(Self {
            header,
            pid,
            tid,
            comm,
        })
    }
}

/// Generic event enum
#[derive(Debug, Clone)]
pub enum Event {
    Sample(SampleEvent),
    Mmap(MmapEvent),
    Comm(CommEvent),
}

/// Writer for perf.data files
pub struct PerfDataWriter<W: Write> {
    writer: W,
    header: Header,
    header_written: bool,
}

impl<W: Write> PerfDataWriter<W> {
    /// Create new perf.data writer
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            header: Header::default(),
            header_written: false,
        }
    }

    /// Write the file header
    pub fn write_header(&mut self) -> io::Result<()> {
        if self.header_written {
            return Ok(());
        }
        self.header.write_to(&mut self.writer)?;
        self.header_written = true;
        Ok(())
    }

    /// Write a sample event
    pub fn write_sample(&mut self, event: &SampleEvent) -> io::Result<()> {
        self.ensure_header()?;
        event.write_to(&mut self.writer)?;
        self.header.sample_count += 1;
        Ok(())
    }

    /// Write a mmap event
    pub fn write_mmap(&mut self, event: &MmapEvent) -> io::Result<()> {
        self.ensure_header()?;
        event.write_to(&mut self.writer)?;
        self.header.mmap_count += 1;
        Ok(())
    }

    /// Write a comm event
    pub fn write_comm(&mut self, event: &CommEvent) -> io::Result<()> {
        self.ensure_header()?;
        event.write_to(&mut self.writer)?;
        self.header.comm_count += 1;
        Ok(())
    }

    /// Write generic event
    pub fn write_event(&mut self, event: &Event) -> io::Result<()> {
        match event {
            Event::Sample(e) => self.write_sample(e),
            Event::Mmap(e) => self.write_mmap(e),
            Event::Comm(e) => self.write_comm(e),
        }
    }

    /// Finalize the file by updating the header
    pub fn finalize(mut self) -> io::Result<()> {
        if !self.header_written {
            return Ok(());
        }

        // Seek back to update header counts
        // Note: This requires W to implement Seek, which we'll handle separately
        // For now, we just flush
        self.writer.flush()
    }

    fn ensure_header(&mut self) -> io::Result<()> {
        if !self.header_written {
            self.write_header()?;
        }
        Ok(())
    }

    /// Get current header statistics
    pub fn stats(&self) -> &Header {
        &self.header
    }
}

impl PerfDataWriter<File> {
    /// Create a writer for a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self::new(file))
    }

    /// Finalize and update header with correct counts
    pub fn finalize_with_header_update(mut self) -> io::Result<()> {
        use std::io::Seek;

        self.writer.flush()?;

        // Seek to beginning and rewrite header with updated counts
        self.writer.seek(std::io::SeekFrom::Start(0))?;
        self.header.write_to(&mut self.writer)?;
        self.writer.flush()?;

        Ok(())
    }
}

/// Reader for perf.data files
pub struct PerfDataReader<R: Read> {
    reader: R,
    header: Header,
    events_read: u64,
}

impl<R: Read> PerfDataReader<R> {
    /// Create new perf.data reader
    pub fn new(mut reader: R) -> Result<Self, PerfError> {
        let header = Header::read_from(&mut reader)?;
        header.validate()?;

        Ok(Self {
            reader,
            header,
            events_read: 0,
        })
    }

    /// Get file header
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Read next event
    pub fn read_event(&mut self) -> Result<Option<Event>, PerfError> {
        let total_events =
            self.header.sample_count + self.header.mmap_count + self.header.comm_count;

        if total_events > 0 && self.events_read >= total_events {
            return Ok(None);
        }

        let header = match EventHeader::read_from(&mut self.reader) {
            Ok(h) => h,
            Err(PerfError::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        };

        let event = match header.event_type {
            EventType::Sample => {
                let sample = SampleEvent::read_from(header, &mut self.reader)?;
                Event::Sample(sample)
            }
            EventType::Mmap => {
                let mmap = MmapEvent::read_from(header, &mut self.reader)?;
                Event::Mmap(mmap)
            }
            EventType::Comm => {
                let comm = CommEvent::read_from(header, &mut self.reader)?;
                Event::Comm(comm)
            }
        };

        self.events_read += 1;
        Ok(Some(event))
    }

    /// Read all events
    pub fn read_all_events(&mut self) -> Result<Vec<Event>, PerfError> {
        let mut events = Vec::new();
        while let Some(event) = self.read_event()? {
            events.push(event);
        }
        Ok(events)
    }
}

impl PerfDataReader<File> {
    /// Create a reader for a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, PerfError> {
        let file = File::open(path)?;
        Self::new(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;

    #[test]
    fn test_header_roundtrip() {
        let mut header = Header::default();
        header.sample_count = 100;
        header.mmap_count = 10;
        header.comm_count = 5;

        let mut buffer = Vec::new();
        header.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_header = Header::read_from(&mut cursor).unwrap();

        assert_eq!(header.magic, read_header.magic);
        assert_eq!(header.version, read_header.version);
        assert_eq!(header.sample_count, read_header.sample_count);
        assert_eq!(header.mmap_count, read_header.mmap_count);
        assert_eq!(header.comm_count, read_header.comm_count);
    }

    #[test]
    fn test_header_validation() {
        let header = Header::default();
        assert!(header.validate().is_ok());

        let mut bad_header = header.clone();
        bad_header.magic = *b"BADXXXXX";
        assert!(bad_header.validate().is_err());

        let mut bad_version = header.clone();
        bad_version.version = 999;
        assert!(bad_version.validate().is_err());
    }

    #[test]
    fn test_event_type_conversion() {
        assert_eq!(EventType::try_from(1).unwrap(), EventType::Mmap);
        assert_eq!(EventType::try_from(2).unwrap(), EventType::Comm);
        assert_eq!(EventType::try_from(3).unwrap(), EventType::Sample);
        assert!(EventType::try_from(99).is_err());
    }

    #[test]
    fn test_sample_event_roundtrip() {
        let sample = SampleEvent::new(
            123456789,
            0x7fff12345678,
            1234,
            5678,
            1000,
            vec![0x7fff11111111, 0x7fff22222222, 0x7fff33333333],
        );

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let header = EventHeader::read_from(&mut cursor).unwrap();
        let read_sample = SampleEvent::read_from(header, &mut cursor).unwrap();

        assert_eq!(sample.ip, read_sample.ip);
        assert_eq!(sample.pid, read_sample.pid);
        assert_eq!(sample.tid, read_sample.tid);
        assert_eq!(sample.period, read_sample.period);
        assert_eq!(sample.callchain, read_sample.callchain);
    }

    #[test]
    fn test_mmap_event_roundtrip() {
        let mmap = MmapEvent::new(
            123456789,
            1234,
            5678,
            0x7fff00000000,
            0x10000,
            0x1000,
            "/lib/x86_64-linux-gnu/libc.so.6".to_string(),
        );

        let mut buffer = Vec::new();
        mmap.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let header = EventHeader::read_from(&mut cursor).unwrap();
        let read_mmap = MmapEvent::read_from(header, &mut cursor).unwrap();

        assert_eq!(mmap.pid, read_mmap.pid);
        assert_eq!(mmap.tid, read_mmap.tid);
        assert_eq!(mmap.addr, read_mmap.addr);
        assert_eq!(mmap.len, read_mmap.len);
        assert_eq!(mmap.pgoff, read_mmap.pgoff);
        assert_eq!(mmap.filename, read_mmap.filename);
    }

    #[test]
    fn test_comm_event_roundtrip() {
        let comm = CommEvent::new(123456789, 1234, 5678, "test-command".to_string());

        let mut buffer = Vec::new();
        comm.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let header = EventHeader::read_from(&mut cursor).unwrap();
        let read_comm = CommEvent::read_from(header, &mut cursor).unwrap();

        assert_eq!(comm.pid, read_comm.pid);
        assert_eq!(comm.tid, read_comm.tid);
        assert_eq!(comm.comm, read_comm.comm);
    }

    #[test]
    fn test_writer_and_reader() {
        let mut buffer = Vec::new();

        {
            let mut writer = PerfDataWriter::new(&mut buffer);

            let sample = SampleEvent::new(1000, 0x1000, 1234, 1234, 1, vec![0x2000, 0x3000]);
            writer.write_sample(&sample).unwrap();

            let mmap = MmapEvent::new(
                2000,
                1234,
                1234,
                0x10000000,
                0x1000,
                0,
                "test.bin".to_string(),
            );
            writer.write_mmap(&mmap).unwrap();

            let comm = CommEvent::new(3000, 1234, 1234, "test".to_string());
            writer.write_comm(&comm).unwrap();
        }

        let cursor = Cursor::new(buffer);
        let mut reader = PerfDataReader::new(cursor).unwrap();

        // In-memory buffer writes header first with zero counts.
        // Use test_file_roundtrip for full header validation.
        let events = reader.read_all_events().unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_file_roundtrip() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write events to file
        {
            let mut writer = PerfDataWriter::from_path(path).unwrap();

            for i in 0..10 {
                let sample = SampleEvent::new(
                    1000 + i,
                    0x1000 + i as u64,
                    1234,
                    1234,
                    1,
                    vec![0x2000 + i as u64, 0x3000 + i as u64],
                );
                writer.write_sample(&sample).unwrap();
            }

            let mmap = MmapEvent::new(
                2000,
                1234,
                1234,
                0x10000000,
                0x1000,
                0,
                "test.bin".to_string(),
            );
            writer.write_mmap(&mmap).unwrap();

            let comm = CommEvent::new(3000, 1234, 1234, "test".to_string());
            writer.write_comm(&comm).unwrap();

            writer.finalize_with_header_update().unwrap();
        }

        // Read events back
        {
            let mut reader = PerfDataReader::from_path(path).unwrap();

            assert_eq!(reader.header().sample_count, 10);
            assert_eq!(reader.header().mmap_count, 1);
            assert_eq!(reader.header().comm_count, 1);

            let events = reader.read_all_events().unwrap();
            assert_eq!(events.len(), 12); // 10 samples + 1 mmap + 1 comm

            // Verify first sample
            if let Event::Sample(sample) = &events[0] {
                assert_eq!(sample.ip, 0x1000);
                assert_eq!(sample.callchain.len(), 2);
            } else {
                panic!("Expected sample event");
            }
        }
    }

    #[test]
    fn test_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write empty file
        {
            let writer = PerfDataWriter::from_path(path).unwrap();
            writer.finalize_with_header_update().unwrap();
        }

        // Read empty file
        {
            let mut reader = PerfDataReader::from_path(path).unwrap();
            assert_eq!(reader.header().sample_count, 0);
            assert_eq!(reader.header().mmap_count, 0);
            assert_eq!(reader.header().comm_count, 0);

            let events = reader.read_all_events().unwrap();
            assert_eq!(events.len(), 0);
        }
    }

    #[test]
    fn test_large_callchain() {
        let large_callchain: Vec<u64> = (0..100).map(|i| 0x1000 + i).collect();

        let sample = SampleEvent::new(1000, 0x1000, 1234, 1234, 1, large_callchain.clone());

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let header = EventHeader::read_from(&mut cursor).unwrap();
        let read_sample = SampleEvent::read_from(header, &mut cursor).unwrap();

        assert_eq!(read_sample.callchain.len(), 100);
        assert_eq!(read_sample.callchain, large_callchain);
    }

    #[test]
    fn test_long_filename() {
        let long_filename =
            "/very/long/path/to/some/library/that/exceeds/normal/length/library.so.1.2.3";

        let mmap = MmapEvent::new(
            1000,
            1234,
            1234,
            0x1000,
            0x1000,
            0,
            long_filename.to_string(),
        );

        let mut buffer = Vec::new();
        mmap.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let header = EventHeader::read_from(&mut cursor).unwrap();
        let read_mmap = MmapEvent::read_from(header, &mut cursor).unwrap();

        assert_eq!(read_mmap.filename, long_filename);
    }
}
