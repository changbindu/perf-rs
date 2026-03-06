use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::error::PerfError;

const MAGIC: &[u8; 8] = b"PERFRS01";
const VERSION: u32 = 1;
const HEADER_SIZE: u32 = 64;
const EVENT_HEADER_SIZE: u16 = 24;

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

#[derive(Debug, Clone)]
pub struct Header {
    pub magic: [u8; 8],
    pub version: u32,
    pub header_size: u32,
    pub sample_count: u64,
    pub mmap_count: u64,
    pub comm_count: u64,
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

#[derive(Debug, Clone)]
pub struct EventHeader {
    pub event_type: EventType,
    pub size: u16,
    pub time: u64,
    pub reserved: u32,
}

impl EventHeader {
    pub fn new(event_type: EventType, size: u16, time: u64) -> Self {
        Self {
            event_type,
            size,
            time,
            reserved: 0,
        }
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&(self.event_type as u16).to_le_bytes())?;
        writer.write_all(&self.size.to_le_bytes())?;
        writer.write_all(&self.time.to_le_bytes())?;
        writer.write_all(&self.reserved.to_le_bytes())?;
        Ok(())
    }

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

#[derive(Debug, Clone)]
pub struct SampleEvent {
    pub header: EventHeader,
    pub ip: u64,
    pub pid: u32,
    pub tid: u32,
    pub period: u64,
    pub callchain: Vec<u64>,
    pub cpu: Option<u32>,
}

impl SampleEvent {
    pub fn new(
        time: u64,
        ip: u64,
        pid: u32,
        tid: u32,
        period: u64,
        callchain: Vec<u64>,
        cpu: Option<u32>,
    ) -> Self {
        let callchain_len = callchain.len() as u32;
        let cpu_size = if cpu.is_some() { 4u16 } else { 0u16 };
        let size = EVENT_HEADER_SIZE + 8 + 4 + 4 + 8 + 4 + (callchain_len as u16 * 8) + cpu_size;

        let header = EventHeader::new(EventType::Sample, size, time);

        Self {
            header,
            ip,
            pid,
            tid,
            period,
            callchain,
            cpu,
        }
    }

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
        if let Some(cpu) = self.cpu {
            writer.write_all(&cpu.to_le_bytes())?;
        }
        Ok(())
    }

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

        let expected_size =
            EVENT_HEADER_SIZE as usize + 8 + 4 + 4 + 8 + 4 + (callchain_len as usize * 8);
        let cpu = if header.size as usize > expected_size {
            reader.read_exact(&mut buf4)?;
            Some(u32::from_le_bytes(buf4))
        } else {
            None
        };

        Ok(Self {
            header,
            ip,
            pid,
            tid,
            period,
            callchain,
            cpu,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MmapEvent {
    pub header: EventHeader,
    pub pid: u32,
    pub tid: u32,
    pub addr: u64,
    pub len: u64,
    pub pgoff: u64,
    pub filename: String,
}

impl MmapEvent {
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

        let data_size = header.size as usize - EVENT_HEADER_SIZE as usize - 4 - 4 - 8 - 8 - 8;
        let mut filename_bytes = vec![0u8; data_size];
        reader.read_exact(&mut filename_bytes)?;

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

#[derive(Debug, Clone)]
pub struct CommEvent {
    pub header: EventHeader,
    pub pid: u32,
    pub tid: u32,
    pub comm: String,
}

impl CommEvent {
    pub fn read_from<R: Read>(header: EventHeader, reader: &mut R) -> io::Result<Self> {
        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        let pid = u32::from_le_bytes(buf4);

        reader.read_exact(&mut buf4)?;
        let tid = u32::from_le_bytes(buf4);

        let data_size = header.size as usize - EVENT_HEADER_SIZE as usize - 4 - 4;
        let mut comm_bytes = vec![0u8; data_size];
        reader.read_exact(&mut comm_bytes)?;

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

#[derive(Debug, Clone)]
pub enum Event {
    Sample(SampleEvent),
    Mmap(MmapEvent),
    Comm(CommEvent),
}

pub struct PerfDataWriter<W: Write> {
    writer: W,
    header: Header,
    header_written: bool,
}

impl<W: Write> PerfDataWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            header: Header::default(),
            header_written: false,
        }
    }

    pub fn write_header(&mut self) -> io::Result<()> {
        if self.header_written {
            return Ok(());
        }
        self.header.write_to(&mut self.writer)?;
        self.header_written = true;
        Ok(())
    }

    pub fn write_sample(&mut self, event: &SampleEvent) -> io::Result<()> {
        self.ensure_header()?;
        event.write_to(&mut self.writer)?;
        self.header.sample_count += 1;
        Ok(())
    }

    fn ensure_header(&mut self) -> io::Result<()> {
        if !self.header_written {
            self.write_header()?;
        }
        Ok(())
    }
}

impl PerfDataWriter<File> {
    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self::new(file))
    }

    pub fn finalize_with_header_update(mut self) -> io::Result<()> {
        use std::io::Seek;

        self.writer.flush()?;
        self.writer.seek(std::io::SeekFrom::Start(0))?;
        self.header.write_to(&mut self.writer)?;
        self.writer.flush()?;

        Ok(())
    }
}

pub struct PerfDataReader<R: Read> {
    reader: R,
    header: Header,
    events_read: u64,
}

impl<R: Read> PerfDataReader<R> {
    pub fn new(mut reader: R) -> Result<Self, PerfError> {
        let header = Header::read_from(&mut reader)?;
        header.validate()?;

        Ok(Self {
            reader,
            header,
            events_read: 0,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

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

    pub fn read_all_events(&mut self) -> Result<Vec<Event>, PerfError> {
        let mut events = Vec::new();
        while let Some(event) = self.read_event()? {
            events.push(event);
        }
        Ok(events)
    }
}

impl PerfDataReader<File> {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, PerfError> {
        let file = File::open(path)?;
        Self::new(file)
    }
}
