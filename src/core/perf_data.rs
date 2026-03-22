//! Linux perf.data file format implementation
//!
//! This module implements the Linux perf.data file format used by the perf tool
//! for performance monitoring data. The format follows the Linux kernel specification.
//!
//! Reference: Linux kernel source, tools/perf/util/header.h and include/uapi/linux/perf_event.h

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::PerfError;

// Constants

/// Magic number for perf.data files (ASCII "PERFILE2" as little-endian u64)
pub const PERF_FILE_MAGIC: u64 = 0x32454C4946524550;

pub const PERF_FILE_MAGIC_BYTES: &[u8; 8] = b"PERFILE2";

/// Size of the perf_file_header structure (104 bytes)
pub const PERF_FILE_HEADER_SIZE: u64 = 104;

/// Size of perf_event_attr in kernel 6.17
pub const PERF_ATTR_SIZE_VER8: u32 = 136;

/// Event header size (8 bytes: u32 type + u16 misc + u16 size)
pub const PERF_EVENT_HEADER_SIZE: u16 = 8;

/// FINISHED_ROUND event size (8 bytes - just the event header)
pub const PERF_FINISHED_ROUND_SIZE: u16 = 8;

// Perf record types
pub const PERF_RECORD_MMAP: u32 = 1;
pub const PERF_RECORD_LOST: u32 = 2;
pub const PERF_RECORD_COMM: u32 = 3;
pub const PERF_RECORD_EXIT: u32 = 4;
pub const PERF_RECORD_THROTTLE: u32 = 5;
pub const PERF_RECORD_UNTHROTTLE: u32 = 6;
pub const PERF_RECORD_FORK: u32 = 7;
pub const PERF_RECORD_READ: u32 = 8;
pub const PERF_RECORD_SAMPLE: u32 = 9;
pub const PERF_RECORD_MMAP2: u32 = 10;
pub const PERF_RECORD_AUX: u32 = 11;
pub const PERF_RECORD_ITRACE_START: u32 = 12;
pub const PERF_RECORD_LOST_SAMPLES: u32 = 13;
pub const PERF_RECORD_SWITCH: u32 = 14;
pub const PERF_RECORD_SWITCH_CPU_WIDE: u32 = 15;
pub const PERF_RECORD_NAMESPACES: u32 = 16;
pub const PERF_RECORD_KSYMBOL: u32 = 17;
pub const PERF_RECORD_BPF_EVENT: u32 = 18;
pub const PERF_RECORD_CGROUP: u32 = 19;
pub const PERF_RECORD_TEXT_POKE: u32 = 20;
pub const PERF_RECORD_AUX_OUTPUT_HW_ID: u32 = 21;
pub const PERF_RECORD_CALLCHAIN_DEFERRED: u32 = 22;
pub const PERF_RECORD_HEADER_ATTR: u32 = 64;
pub const PERF_RECORD_HEADER_EVENT_TYPE: u32 = 65;
pub const PERF_RECORD_HEADER_TRACING_DATA: u32 = 66;
pub const PERF_RECORD_HEADER_BUILD_ID: u32 = 67;
pub const PERF_RECORD_FINISHED_ROUND: u32 = 68;
pub const PERF_RECORD_ID_INDEX: u32 = 69;
pub const PERF_RECORD_AUXTRACE_INFO: u32 = 70;
pub const PERF_RECORD_AUXTRACE: u32 = 71;
pub const PERF_RECORD_AUXTRACE_ERROR: u32 = 72;
pub const PERF_RECORD_HEADER_FEATURE: u32 = 80;
pub const PERF_RECORD_COMPRESSED: u32 = 81;
pub const PERF_RECORD_FINISHED_INIT: u32 = 82;
pub const PERF_RECORD_COMPRESSED2: u32 = 83;

// File Header Structures

/// Perf file section descriptor (16 bytes)
#[derive(Debug, Clone, Copy, Default)]
pub struct PerfFileSection {
    /// Offset from start of file
    pub offset: u64,
    /// Size of the section in bytes
    pub size: u64,
}

impl PerfFileSection {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u64::<LittleEndian>(self.offset)?;
        writer.write_u64::<LittleEndian>(self.size)?;
        Ok(())
    }
}

/// Perf file header (104 bytes)
///
/// This is the first structure in a perf.data file.
#[derive(Debug, Clone)]
pub struct PerfFileHeader {
    pub magic: u64,
    pub size: u64,
    pub attr_size: u64,
    pub attrs: PerfFileSection,
    pub data: PerfFileSection,
    pub event_types: PerfFileSection,
    pub flags: u64,
    pub flags1: [u64; 3],
}

impl Default for PerfFileHeader {
    fn default() -> Self {
        Self {
            magic: PERF_FILE_MAGIC,
            size: PERF_FILE_HEADER_SIZE,
            attr_size: PERF_ATTR_SIZE_VER8 as u64,
            attrs: PerfFileSection::default(),
            data: PerfFileSection::default(),
            event_types: PerfFileSection::default(),
            flags: 0,
            flags1: [0; 3],
        }
    }
}

impl PerfFileHeader {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        writer.write_all(b"PERFILE2")?;
        writer.write_u64::<LittleEndian>(self.size)?;
        writer.write_u64::<LittleEndian>(self.attr_size)?;
        self.attrs.write_to(writer)?;
        self.data.write_to(writer)?;
        self.event_types.write_to(writer)?;
        writer.write_u64::<LittleEndian>(self.flags)?;
        for flag in &self.flags1 {
            writer.write_u64::<LittleEndian>(*flag)?;
        }

        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let mut magic_bytes = [0u8; 8];
        reader.read_exact(&mut magic_bytes)?;

        if &magic_bytes != b"PERFILE2" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid magic number: expected 'PERFILE2', got '{:?}'",
                    magic_bytes
                ),
            ));
        }

        let magic = u64::from_le_bytes(magic_bytes);
        let size = reader.read_u64::<LittleEndian>()?;
        let attr_size = reader.read_u64::<LittleEndian>()?;

        let attrs = PerfFileSection {
            offset: reader.read_u64::<LittleEndian>()?,
            size: reader.read_u64::<LittleEndian>()?,
        };

        let data = PerfFileSection {
            offset: reader.read_u64::<LittleEndian>()?,
            size: reader.read_u64::<LittleEndian>()?,
        };

        let event_types = PerfFileSection {
            offset: reader.read_u64::<LittleEndian>()?,
            size: reader.read_u64::<LittleEndian>()?,
        };

        let flags = reader.read_u64::<LittleEndian>()?;
        let mut flags1 = [0u64; 3];
        for flag in flags1.iter_mut() {
            *flag = reader.read_u64::<LittleEndian>()?;
        }

        Ok(Self {
            magic,
            size,
            attr_size,
            attrs,
            data,
            event_types,
            flags,
            flags1,
        })
    }

    pub fn validate(&self) -> Result<(), PerfError> {
        if self.size != PERF_FILE_HEADER_SIZE {
            return Err(PerfError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid header size: expected {}, got {}",
                    PERF_FILE_HEADER_SIZE, self.size
                ),
            )));
        }
        Ok(())
    }
}

// perf_event_attr Structure

/// perf_event_attr structure (136 bytes in kernel 6.17)
///
/// This structure describes the configuration of a performance counter.
#[derive(Debug, Clone)]
pub struct PerfEventAttr {
    /// Major type: hardware/software/tracepoint/etc
    pub type_: u32,
    /// Size of this structure
    pub size: u32,
    /// Type-specific configuration
    pub config: u64,
    /// Sampling period or frequency
    pub sample_period_or_freq: u64,
    /// What to include in samples
    pub sample_type: u64,
    /// Format of read() data
    pub read_format: u64,
    /// Bit fields for event attributes
    pub bitfields: u64,
    /// Wake up every n events or bytes before wakeup
    pub wakeup_events_or_watermark: u32,
    /// Breakpoint type
    pub bp_type: u32,
    /// Breakpoint address or config1
    pub config1: u64,
    /// Breakpoint length or config2
    pub config2: u64,
    /// Branch sample type
    pub branch_sample_type: u64,
    /// User registers to sample on overflow
    pub sample_regs_user: u64,
    /// Size of user stack to dump on overflow
    pub sample_stack_user: u32,
    /// Clock to use for time fields
    pub clockid: i32,
    /// Registers to sample on interrupt
    pub sample_regs_intr: u64,
    /// Size of AUX area
    pub aux_watermark: u32,
    /// Maximum stack depth for callchain
    pub sample_max_stack: u16,
    pub _reserved_2: u16,
    /// Size of AUX area samples
    pub aux_sample_size: u32,
    /// AUX area actions
    pub aux_action: u32,
    /// Signal data
    pub sig_data: u64,
    /// Config 3
    pub config3: u64,
}

impl Default for PerfEventAttr {
    fn default() -> Self {
        Self {
            type_: 0,
            size: PERF_ATTR_SIZE_VER8,
            config: 0,
            sample_period_or_freq: 0,
            sample_type: 0,
            read_format: 0,
            bitfields: 0,
            wakeup_events_or_watermark: 0,
            bp_type: 0,
            config1: 0,
            config2: 0,
            branch_sample_type: 0,
            sample_regs_user: 0,
            sample_stack_user: 0,
            clockid: -1,
            sample_regs_intr: 0,
            aux_watermark: 0,
            sample_max_stack: 0,
            _reserved_2: 0,
            aux_sample_size: 0,
            aux_action: 0,
            sig_data: 0,
            config3: 0,
        }
    }
}

impl PerfEventAttr {
    pub fn new(type_: u32, config: u64, sample_type: u64) -> Self {
        let mut attr = Self::default();
        attr.type_ = type_;
        attr.config = config;
        attr.sample_type = sample_type;
        attr
    }

    pub fn with_sample_period(mut self, period: u64) -> Self {
        self.sample_period_or_freq = period;
        self
    }

    pub fn with_sample_freq(mut self, freq: u64) -> Self {
        self.sample_period_or_freq = freq;
        self.set_bitfield_bit(11, true);
        self
    }

    pub fn with_sample_id_all(mut self, enabled: bool) -> Self {
        self.set_bitfield_bit(18, enabled);
        self
    }

    pub fn with_comm(mut self, enabled: bool) -> Self {
        self.set_bitfield_bit(8, enabled);
        self
    }

    pub fn with_mmap(mut self, enabled: bool) -> Self {
        self.set_bitfield_bit(7, enabled);
        self
    }

    fn set_bitfield_bit(&mut self, bit: u32, value: bool) {
        if value {
            self.bitfields |= 1 << bit;
        } else {
            self.bitfields &= !(1 << bit);
        }
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        writer.write_u32::<LittleEndian>(self.type_)?;
        writer.write_u32::<LittleEndian>(self.size)?;
        writer.write_u64::<LittleEndian>(self.config)?;
        writer.write_u64::<LittleEndian>(self.sample_period_or_freq)?;
        writer.write_u64::<LittleEndian>(self.sample_type)?;
        writer.write_u64::<LittleEndian>(self.read_format)?;
        writer.write_u64::<LittleEndian>(self.bitfields)?;
        writer.write_u32::<LittleEndian>(self.wakeup_events_or_watermark)?;
        writer.write_u32::<LittleEndian>(self.bp_type)?;
        writer.write_u64::<LittleEndian>(self.config1)?;
        writer.write_u64::<LittleEndian>(self.config2)?;
        writer.write_u64::<LittleEndian>(self.branch_sample_type)?;
        writer.write_u64::<LittleEndian>(self.sample_regs_user)?;
        writer.write_u32::<LittleEndian>(self.sample_stack_user)?;
        writer.write_i32::<LittleEndian>(self.clockid)?;
        writer.write_u64::<LittleEndian>(self.sample_regs_intr)?;
        writer.write_u32::<LittleEndian>(self.aux_watermark)?;
        writer.write_u16::<LittleEndian>(self.sample_max_stack)?;
        writer.write_u16::<LittleEndian>(self._reserved_2)?;
        writer.write_u32::<LittleEndian>(self.aux_sample_size)?;
        writer.write_u32::<LittleEndian>(self.aux_action)?;
        writer.write_u64::<LittleEndian>(self.sig_data)?;
        writer.write_u64::<LittleEndian>(self.config3)?;

        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};

        Ok(Self {
            type_: reader.read_u32::<LittleEndian>()?,
            size: reader.read_u32::<LittleEndian>()?,
            config: reader.read_u64::<LittleEndian>()?,
            sample_period_or_freq: reader.read_u64::<LittleEndian>()?,
            sample_type: reader.read_u64::<LittleEndian>()?,
            read_format: reader.read_u64::<LittleEndian>()?,
            bitfields: reader.read_u64::<LittleEndian>()?,
            wakeup_events_or_watermark: reader.read_u32::<LittleEndian>()?,
            bp_type: reader.read_u32::<LittleEndian>()?,
            config1: reader.read_u64::<LittleEndian>()?,
            config2: reader.read_u64::<LittleEndian>()?,
            branch_sample_type: reader.read_u64::<LittleEndian>()?,
            sample_regs_user: reader.read_u64::<LittleEndian>()?,
            sample_stack_user: reader.read_u32::<LittleEndian>()?,
            clockid: reader.read_i32::<LittleEndian>()?,
            sample_regs_intr: reader.read_u64::<LittleEndian>()?,
            aux_watermark: reader.read_u32::<LittleEndian>()?,
            sample_max_stack: reader.read_u16::<LittleEndian>()?,
            _reserved_2: reader.read_u16::<LittleEndian>()?,
            aux_sample_size: reader.read_u32::<LittleEndian>()?,
            aux_action: reader.read_u32::<LittleEndian>()?,
            sig_data: reader.read_u64::<LittleEndian>()?,
            config3: reader.read_u64::<LittleEndian>()?,
        })
    }
}

// Event Header and Records

/// Common header for all event records (8 bytes: u32 type + u16 misc + u16 size)
#[derive(Debug, Clone, Copy)]
pub struct PerfEventHeader {
    /// Event type (PERF_RECORD_*)
    pub type_: u32,
    /// Additional flags
    pub misc: u16,
    /// Total size of the event record including header
    pub size: u16,
}

impl PerfEventHeader {
    pub fn new(type_: u32, size: u16) -> Self {
        Self {
            type_,
            misc: 0,
            size,
        }
    }

    pub fn with_misc(mut self, misc: u16) -> Self {
        self.misc = misc;
        self
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.type_)?;
        writer.write_u16::<LittleEndian>(self.misc)?;
        writer.write_u16::<LittleEndian>(self.size)?;
        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            type_: reader.read_u32::<LittleEndian>()?,
            misc: reader.read_u16::<LittleEndian>()?,
            size: reader.read_u16::<LittleEndian>()?,
        })
    }
}

/// FINISHED_ROUND event (type 68, no payload)
///
/// Marks a point in the event stream where no reordering occurs across this boundary.
#[derive(Debug, Clone, Copy)]
pub struct FinishedRoundEvent {
    pub header: PerfEventHeader,
}

impl Default for FinishedRoundEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl FinishedRoundEvent {
    pub fn new() -> Self {
        Self {
            header: PerfEventHeader::new(PERF_RECORD_FINISHED_ROUND, PERF_FINISHED_ROUND_SIZE),
        }
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.header.write_to(writer)?;
        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let header = PerfEventHeader::read_from(reader)?;
        Ok(Self { header })
    }

    pub fn read_from_header<R: Read>(_reader: &mut R, header: PerfEventHeader) -> io::Result<Self> {
        Ok(Self { header })
    }
}

// Sample type flags for PERF_RECORD_SAMPLE
pub const PERF_SAMPLE_IP: u64 = 1 << 0;
pub const PERF_SAMPLE_TID: u64 = 1 << 1;
pub const PERF_SAMPLE_TIME: u64 = 1 << 2;
pub const PERF_SAMPLE_ADDR: u64 = 1 << 3;
pub const PERF_SAMPLE_READ: u64 = 1 << 4;
pub const PERF_SAMPLE_CALLCHAIN: u64 = 1 << 5;
pub const PERF_SAMPLE_ID: u64 = 1 << 6;
pub const PERF_SAMPLE_CPU: u64 = 1 << 7;
pub const PERF_SAMPLE_PERIOD: u64 = 1 << 8;
pub const PERF_SAMPLE_STREAM_ID: u64 = 1 << 9;
pub const PERF_SAMPLE_RAW: u64 = 1 << 10;
pub const PERF_SAMPLE_IDENTIFIER: u64 = 1 << 16;

/// PERF_RECORD_MMAP event (type 1)
///
/// Records a memory mapping event.
#[derive(Debug, Clone)]
pub struct MmapEvent {
    pub header: PerfEventHeader,
    pub pid: u32,
    pub tid: u32,
    pub addr: u64,
    pub len: u64,
    pub pgoff: u64,
    pub filename: String,
}

impl MmapEvent {
    pub fn new(pid: u32, tid: u32, addr: u64, len: u64, pgoff: u64, filename: String) -> Self {
        let filename_bytes_len = filename.len() + 1;
        let padded_len = ((filename_bytes_len + 7) / 8) * 8;
        let total_size = 8 + 4 + 4 + 8 + 8 + 8 + padded_len;

        Self {
            header: PerfEventHeader::new(PERF_RECORD_MMAP, total_size as u16),
            pid,
            tid,
            addr,
            len,
            pgoff,
            filename,
        }
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        self.header.write_to(writer)?;
        writer.write_u32::<LittleEndian>(self.pid)?;
        writer.write_u32::<LittleEndian>(self.tid)?;
        writer.write_u64::<LittleEndian>(self.addr)?;
        writer.write_u64::<LittleEndian>(self.len)?;
        writer.write_u64::<LittleEndian>(self.pgoff)?;

        write_null_terminated_padded_string(writer, &self.filename, 8)?;

        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let header = PerfEventHeader::read_from(reader)?;
        Self::read_from_header(reader, header)
    }

    pub fn read_from_header<R: Read>(reader: &mut R, header: PerfEventHeader) -> io::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let pid = reader.read_u32::<LittleEndian>()?;
        let tid = reader.read_u32::<LittleEndian>()?;
        let addr = reader.read_u64::<LittleEndian>()?;
        let len = reader.read_u64::<LittleEndian>()?;
        let pgoff = reader.read_u64::<LittleEndian>()?;
        let filename = read_null_terminated_padded_string(reader, 8)?;

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

/// PERF_RECORD_COMM event (type 3)
///
/// Records a command name (exec) event.
#[derive(Debug, Clone)]
pub struct CommEvent {
    pub header: PerfEventHeader,
    pub pid: u32,
    pub tid: u32,
    pub comm: String,
}

impl CommEvent {
    pub fn new(pid: u32, tid: u32, comm: String) -> Self {
        let comm_bytes_len = comm.len() + 1;
        let padded_len = ((comm_bytes_len + 7) / 8) * 8;
        let total_size = 8 + 4 + 4 + padded_len;

        Self {
            header: PerfEventHeader::new(PERF_RECORD_COMM, total_size as u16),
            pid,
            tid,
            comm,
        }
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        self.header.write_to(writer)?;
        writer.write_u32::<LittleEndian>(self.pid)?;
        writer.write_u32::<LittleEndian>(self.tid)?;

        write_null_terminated_padded_string(writer, &self.comm, 8)?;

        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let header = PerfEventHeader::read_from(reader)?;
        Self::read_from_header(reader, header)
    }

    pub fn read_from_header<R: Read>(reader: &mut R, header: PerfEventHeader) -> io::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let pid = reader.read_u32::<LittleEndian>()?;
        let tid = reader.read_u32::<LittleEndian>()?;
        let comm = read_null_terminated_padded_string(reader, 8)?;

        Ok(Self {
            header,
            pid,
            tid,
            comm,
        })
    }
}

/// PERF_RECORD_SAMPLE event (type 9)
///
/// Records a sample event with variable fields based on sample_type.
#[derive(Debug, Clone)]
pub struct SampleEvent {
    pub header: PerfEventHeader,
    pub sample_type: u64,
    pub event_id: u64,
    pub ip: u64,
    pub pid: u32,
    pub tid: u32,
    pub time: u64,
    pub period: u64,
    pub callchain: Option<Vec<u64>>,
    pub cpu: Option<u32>,
}

impl SampleEvent {
    pub fn new(
        sample_type: u64,
        time: u64,
        ip: u64,
        pid: u32,
        tid: u32,
        period: u64,
        callchain: Option<Vec<u64>>,
        cpu: Option<u32>,
        event_id: u64,
    ) -> Self {
        let mut size = 8u16;

        if sample_type & PERF_SAMPLE_IDENTIFIER != 0 {
            size += 8;
        }
        if sample_type & PERF_SAMPLE_IP != 0 {
            size += 8;
        }
        if sample_type & PERF_SAMPLE_TID != 0 {
            size += 8;
        }
        if sample_type & PERF_SAMPLE_TIME != 0 {
            size += 8;
        }
        if sample_type & PERF_SAMPLE_CALLCHAIN != 0 {
            size += 8;
            if let Some(ref cc) = callchain {
                size += (cc.len() as u16) * 8;
            }
        }
        if sample_type & PERF_SAMPLE_PERIOD != 0 {
            size += 8;
        }

        Self {
            header: PerfEventHeader::new(PERF_RECORD_SAMPLE, size),
            sample_type,
            event_id,
            ip,
            pid,
            tid,
            time,
            period,
            callchain,
            cpu,
        }
    }

    fn calculate_size(&self) -> u16 {
        let mut size = 8u16;

        if self.sample_type & PERF_SAMPLE_IDENTIFIER != 0 {
            size += 8;
        }
        if self.sample_type & PERF_SAMPLE_IP != 0 {
            size += 8;
        }
        if self.sample_type & PERF_SAMPLE_TID != 0 {
            size += 8;
        }
        if self.sample_type & PERF_SAMPLE_TIME != 0 {
            size += 8;
        }
        if self.sample_type & PERF_SAMPLE_CALLCHAIN != 0 {
            size += 8;
            if let Some(ref cc) = self.callchain {
                size += (cc.len() as u16) * 8;
            }
        }
        if self.sample_type & PERF_SAMPLE_PERIOD != 0 {
            size += 8;
        }

        size
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        let size = self.calculate_size();
        let header = PerfEventHeader::new(PERF_RECORD_SAMPLE, size);

        header.write_to(writer)?;

        if self.sample_type & PERF_SAMPLE_IDENTIFIER != 0 {
            writer.write_u64::<LittleEndian>(self.event_id)?;
        }

        if self.sample_type & PERF_SAMPLE_IP != 0 {
            writer.write_u64::<LittleEndian>(self.ip)?;
        }

        if self.sample_type & PERF_SAMPLE_TID != 0 {
            writer.write_u32::<LittleEndian>(self.pid)?;
            writer.write_u32::<LittleEndian>(self.tid)?;
        }

        if self.sample_type & PERF_SAMPLE_TIME != 0 {
            writer.write_u64::<LittleEndian>(self.time)?;
        }

        // PERIOD (bit 8) must be written BEFORE CALLCHAIN (bit 5)
        // The field order is defined by the kernel, NOT by bit number
        // See: include/uapi/linux/perf_event.h PERF_RECORD_SAMPLE struct
        if self.sample_type & PERF_SAMPLE_PERIOD != 0 {
            writer.write_u64::<LittleEndian>(self.period)?;
        }

        if self.sample_type & PERF_SAMPLE_CALLCHAIN != 0 {
            let nr = self
                .callchain
                .as_ref()
                .map(|cc| cc.len() as u64)
                .unwrap_or(0);
            writer.write_u64::<LittleEndian>(nr)?;
            if let Some(ref cc) = self.callchain {
                for ip in cc {
                    writer.write_u64::<LittleEndian>(*ip)?;
                }
            }
        }

        Ok(())
    }

    pub fn read_from<R: Read + Seek>(reader: &mut R, sample_type: u64) -> io::Result<Self> {
        let header = PerfEventHeader::read_from(reader)?;
        Self::read_from_header(reader, header, sample_type)
    }

    pub fn read_from_header<R: Read + Seek>(
        reader: &mut R,
        header: PerfEventHeader,
        sample_type: u64,
    ) -> io::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let start_pos = reader.stream_position().unwrap_or(0);
        let payload_size = header.size as usize - PERF_EVENT_HEADER_SIZE as usize;

        let mut ip = 0u64;
        let mut pid = 0u32;
        let mut tid = 0u32;
        let mut time = 0u64;
        let mut period = 0u64;
        let mut callchain = None;

        if sample_type & PERF_SAMPLE_IP != 0 {
            ip = reader.read_u64::<LittleEndian>()?;
        }

        if sample_type & PERF_SAMPLE_TID != 0 {
            pid = reader.read_u32::<LittleEndian>()?;
            tid = reader.read_u32::<LittleEndian>()?;
        }

        if sample_type & PERF_SAMPLE_TIME != 0 {
            time = reader.read_u64::<LittleEndian>()?;
        }

        // PERIOD (bit 8) must be read BEFORE CALLCHAIN (bit 5)
        // The field order is defined by the kernel, NOT by bit number
        if sample_type & PERF_SAMPLE_PERIOD != 0 {
            period = reader.read_u64::<LittleEndian>()?;
        }

        if sample_type & PERF_SAMPLE_CALLCHAIN != 0 {
            let nr = reader.read_u64::<LittleEndian>()? as usize;
            if nr > 0 {
                let mut chain = Vec::with_capacity(nr);
                for _ in 0..nr {
                    chain.push(reader.read_u64::<LittleEndian>()?);
                }
                callchain = Some(chain);
            }
        }

        // Skip any remaining bytes to ensure we're at the correct position
        let current_pos = reader.stream_position().unwrap_or(0);
        let bytes_read = (current_pos - start_pos) as usize;
        let remaining = payload_size.saturating_sub(bytes_read);
        if remaining > 0 {
            let mut skip = vec![0u8; remaining];
            reader.read_exact(&mut skip)?;
        }

        Ok(Self {
            header,
            sample_type,
            event_id: 0,
            ip,
            pid,
            tid,
            time,
            period,
            callchain,
            cpu: None,
        })
    }
}

/// Helper function to write a null-terminated string with padding to specified alignment
fn write_null_terminated_padded_string<W: Write>(
    writer: &mut W,
    s: &str,
    alignment: usize,
) -> io::Result<()> {
    let bytes = s.as_bytes();
    writer.write_all(bytes)?;
    writer.write_all(&[0u8])?; // Null terminator

    // Calculate padding needed
    let total_len = bytes.len() + 1;
    let padding = (alignment - (total_len % alignment)) % alignment;

    if padding > 0 {
        writer_write_padding(writer, padding)?;
    }

    Ok(())
}

/// Helper function to read a null-terminated string with padding to specified alignment
fn read_null_terminated_padded_string<R: Read>(
    reader: &mut R,
    alignment: usize,
) -> io::Result<String> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1];

    loop {
        reader.read_exact(&mut buf)?;
        if buf[0] == 0 {
            break;
        }
        bytes.push(buf[0]);
    }

    let total_len = bytes.len() + 1;
    let padding = (alignment - (total_len % alignment)) % alignment;
    if padding > 0 {
        let mut padding_buf = vec![0u8; padding];
        reader.read_exact(&mut padding_buf)?;
    }

    String::from_utf8(bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))
}

// Writer Implementation

/// Writer for Linux perf.data files
///
/// This writer creates files compatible with the Linux perf tool.
pub struct PerfDataWriter<W: Write + Seek> {
    writer: W,
    header: PerfFileHeader,
    attrs_written: bool,
    data_start_offset: u64,
    data_size: u64,
}

impl<W: Write + Seek> PerfDataWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            header: PerfFileHeader::default(),
            attrs_written: false,
            data_start_offset: 0,
            data_size: 0,
        }
    }

    pub fn initialize(&mut self, attrs: &[PerfEventAttr], ids: &[Vec<u64>]) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        let header_start = self.writer.stream_position()?;
        self.header.write_to(&mut self.writer)?;

        let header_end = self.writer.stream_position()?;
        let ids_start = header_end;

        for id_vec in ids {
            for id in id_vec {
                self.writer.write_u64::<LittleEndian>(*id)?;
            }
        }

        let ids_end = self.writer.stream_position()?;
        let attrs_start = (ids_end + 7) & !7;

        let padding_to_attrs = attrs_start - ids_end;
        if padding_to_attrs > 0 {
            writer_write_padding(&mut self.writer, padding_to_attrs as usize)?;
        }

        // Each attr entry is: perf_event_attr (136 bytes) + ids_offset (8) + ids_size (8) = 152 bytes
        let attr_entry_size = PERF_ATTR_SIZE_VER8 as u64 + 16;
        let mut attrs_size = 0u64;
        for (attr, _id_vec) in attrs.iter().zip(ids.iter()) {
            attr.write_to(&mut self.writer)?;

            let ids_offset = ids_start;
            let ids_size = ids_end - ids_start;

            self.writer.write_u64::<LittleEndian>(ids_offset)?;
            self.writer.write_u64::<LittleEndian>(ids_size)?;
            attrs_size += attr_entry_size;
        }

        let current_pos = self.writer.stream_position()?;
        self.data_start_offset = (current_pos + 7) & !7;

        let padding_to_data = self.data_start_offset - current_pos;
        if padding_to_data > 0 {
            writer_write_padding(&mut self.writer, padding_to_data as usize)?;
        }

        self.header.attr_size = attr_entry_size;
        self.header.attrs.offset = attrs_start;
        self.header.attrs.size = attrs_size;
        self.header.data.offset = self.data_start_offset;
        self.header.data.size = 0;

        self.writer.seek(SeekFrom::Start(header_start))?;
        self.header.write_to(&mut self.writer)?;
        self.writer.seek(SeekFrom::Start(self.data_start_offset))?;

        self.attrs_written = true;
        Ok(())
    }

    pub fn write_finished_round(&mut self) -> io::Result<()> {
        let event = FinishedRoundEvent::new();
        event.write_to(&mut self.writer)?;
        self.data_size += PERF_FINISHED_ROUND_SIZE as u64;

        let current_pos = self.writer.stream_position()?;
        let padding = (8 - (current_pos as usize % 8)) % 8;
        if padding > 0 {
            writer_write_padding(&mut self.writer, padding)?;
            self.data_size += padding as u64;
        }

        Ok(())
    }

    /// Write an event header
    pub fn write_event_header(&mut self, header: &PerfEventHeader) -> io::Result<()> {
        header.write_to(&mut self.writer)?;
        self.data_size += header.size as u64;
        Ok(())
    }

    /// Write an MMAP event
    pub fn write_mmap(&mut self, mmap: &MmapEvent) -> io::Result<()> {
        mmap.write_to(&mut self.writer)?;
        self.data_size += mmap.header.size as u64;
        Ok(())
    }

    /// Write a COMM event
    pub fn write_comm(&mut self, comm: &CommEvent) -> io::Result<()> {
        comm.write_to(&mut self.writer)?;
        self.data_size += comm.header.size as u64;
        Ok(())
    }

    /// Write a SAMPLE event
    pub fn write_sample(&mut self, sample: &SampleEvent) -> io::Result<()> {
        sample.write_to(&mut self.writer)?;
        self.data_size += sample.calculate_size() as u64;

        let current_pos = self.writer.stream_position()?;
        let padding = (8 - (current_pos as usize % 8)) % 8;
        if padding > 0 {
            writer_write_padding(&mut self.writer, padding)?;
            self.data_size += padding as u64;
        }

        Ok(())
    }

    /// Write raw event data
    pub fn write_event_data(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data)?;
        Ok(())
    }

    /// Write padding to align to 8-byte boundary
    pub fn align_to_8_bytes(&mut self) -> io::Result<()> {
        let current_pos = self.writer.stream_position()?;
        let alignment = 8;
        let padding = (alignment - (current_pos as usize % alignment)) % alignment;
        if padding > 0 {
            writer_write_padding(&mut self.writer, padding)?;
            self.data_size += padding as u64;
        }
        Ok(())
    }

    /// Finalize the file (update header with data section size)
    pub fn finalize(mut self) -> io::Result<()> {
        self.header.data.size = self.data_size;

        let header_start = 0;
        self.writer.seek(SeekFrom::Start(header_start))?;
        self.header.write_to(&mut self.writer)?;
        self.writer.flush()?;

        Ok(())
    }
}

/// Write padding bytes (zeros)
fn writer_write_padding<W: Write>(writer: &mut W, count: usize) -> io::Result<()> {
    let padding = vec![0u8; count];
    writer.write_all(&padding)?;
    Ok(())
}

impl PerfDataWriter<File> {
    /// Create a writer from a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self::new(file))
    }
}

// Test Function

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_header_serialization() {
        let header = PerfFileHeader::default();
        let mut buffer = Vec::new();

        header.write_to(&mut buffer).unwrap();

        assert_eq!(&buffer[0..8], b"PERFILE2");

        let size = u64::from_le_bytes([
            buffer[8], buffer[9], buffer[10], buffer[11], buffer[12], buffer[13], buffer[14],
            buffer[15],
        ]);
        assert_eq!(size, 104);

        let mut cursor = Cursor::new(buffer);
        let read_header = PerfFileHeader::read_from(&mut cursor).unwrap();
        assert_eq!(read_header.magic, PERF_FILE_MAGIC);
        assert_eq!(read_header.size, PERF_FILE_HEADER_SIZE);
    }

    #[test]
    fn test_finished_round_event() {
        let event = FinishedRoundEvent::new();
        assert_eq!(event.header.type_, PERF_RECORD_FINISHED_ROUND);
        assert_eq!(event.header.size, PERF_FINISHED_ROUND_SIZE);

        let mut buffer = Vec::new();
        event.write_to(&mut buffer).unwrap();

        assert_eq!(buffer.len(), PERF_FINISHED_ROUND_SIZE as usize);

        // type_ (u32 little-endian): bytes 0-3
        assert_eq!(buffer[0], 68);
        assert_eq!(buffer[1], 0);
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);

        // misc (u16 little-endian): bytes 4-5
        assert_eq!(buffer[4], 0);
        assert_eq!(buffer[5], 0);

        // size (u16 little-endian): bytes 6-7
        assert_eq!(buffer[6], 8);
        assert_eq!(buffer[7], 0);
    }

    #[test]
    fn test_perf_event_attr_default() {
        let attr = PerfEventAttr::default();
        assert_eq!(attr.size, PERF_ATTR_SIZE_VER8);
        assert_eq!(attr.clockid, -1);
    }

    #[test]
    fn test_perf_event_attr_builder() {
        let attr = PerfEventAttr::new(0, 1, 0)
            .with_sample_freq(99)
            .with_sample_id_all(true)
            .with_comm(true)
            .with_mmap(true);

        assert_eq!(attr.type_, 0);
        assert_eq!(attr.config, 1);
        assert_eq!(attr.sample_period_or_freq, 99);
        assert_eq!(attr.bitfields & (1 << 11), 1 << 11);
        assert_eq!(attr.bitfields & (1 << 18), 1 << 18);
        assert_eq!(attr.bitfields & (1 << 8), 1 << 8);
        assert_eq!(attr.bitfields & (1 << 7), 1 << 7);
    }

    #[test]
    fn test_writer_minimal() {
        let mut buffer = Cursor::new(Vec::new());

        let attr = PerfEventAttr::new(0, 0, 0);
        let ids = vec![vec![1u64]];

        let mut writer = PerfDataWriter::new(&mut buffer);
        writer.initialize(&[attr], &ids).unwrap();
        writer.write_finished_round().unwrap();
        writer.align_to_8_bytes().unwrap();
        writer.finalize().unwrap();

        let data = buffer.into_inner();
        assert_eq!(&data[0..8], b"PERFILE2");

        #[cfg(feature = "evidence")]
        {
            let _ = std::fs::write(".sisyphus/evidence/test-output.perf.data", &data);
        }
    }

    #[test]
    fn test_mmap_event() {
        let mmap = MmapEvent::new(
            1234,
            5678,
            0x7f0000000000,
            0x1000,
            0,
            "/usr/bin/ls".to_string(),
        );

        assert_eq!(mmap.header.type_, PERF_RECORD_MMAP);
        assert_eq!(mmap.pid, 1234);
        assert_eq!(mmap.tid, 5678);
        assert_eq!(mmap.addr, 0x7f0000000000);
        assert_eq!(mmap.len, 0x1000);

        let mut buffer = Vec::new();
        mmap.write_to(&mut buffer).unwrap();

        // Verify header
        assert_eq!(buffer[0], 1); // MMAP type
        assert_eq!(buffer[1], 0);
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);
        assert_eq!(
            u16::from_le_bytes([buffer[6], buffer[7]]),
            buffer.len() as u16
        );

        // Verify pid/tid (starts at offset 8)
        let pid = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
        let tid = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
        assert_eq!(pid, 1234);
        assert_eq!(tid, 5678);

        // Verify filename with null terminator
        let filename_start = 8 + 4 + 4 + 8 + 8 + 8;
        let filename_bytes = &buffer[filename_start..];
        assert_eq!(&filename_bytes[0..12], b"/usr/bin/ls\0");
    }

    #[test]
    fn test_mmap_event_alignment() {
        let short_filename = "ls";
        let mmap = MmapEvent::new(1234, 5678, 0, 0x1000, 0, short_filename.to_string());

        let mut buffer = Vec::new();
        mmap.write_to(&mut buffer).unwrap();

        let filename_start = 8 + 4 + 4 + 8 + 8 + 8;
        assert_eq!(&buffer[filename_start..filename_start + 2], b"ls");
        assert_eq!(buffer[filename_start + 2], 0);
        assert_eq!(&buffer[filename_start + 3..filename_start + 8], &[0u8; 5]);

        // Total buffer size
        assert_eq!(buffer.len(), 48);
        assert_eq!(mmap.header.size as usize, buffer.len());
    }

    #[test]
    fn test_comm_event() {
        let comm = CommEvent::new(1234, 5678, "test-program".to_string());

        assert_eq!(comm.header.type_, PERF_RECORD_COMM);
        assert_eq!(comm.pid, 1234);
        assert_eq!(comm.tid, 5678);
        assert_eq!(comm.comm, "test-program");

        let mut buffer = Vec::new();
        comm.write_to(&mut buffer).unwrap();

        // Verify header
        assert_eq!(buffer[0], 3); // COMM type
        assert_eq!(buffer[1], 0);
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);

        // Verify pid/tid (starts at offset 8)
        let pid = u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
        let tid = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
        assert_eq!(pid, 1234);
        assert_eq!(tid, 5678);

        // Verify comm string with null terminator
        let comm_start = 8 + 4 + 4;
        let comm_bytes = &buffer[comm_start..];
        assert_eq!(&comm_bytes[0..13], b"test-program\0");
    }

    #[test]
    fn test_comm_event_alignment() {
        let short_comm = "sh";
        let comm = CommEvent::new(1234, 5678, short_comm.to_string());

        let mut buffer = Vec::new();
        comm.write_to(&mut buffer).unwrap();

        let comm_start = 8 + 4 + 4;
        assert_eq!(&buffer[comm_start..comm_start + 2], b"sh");
        assert_eq!(buffer[comm_start + 2], 0);
        assert_eq!(&buffer[comm_start + 3..comm_start + 8], &[0u8; 5]);
        assert_eq!(&buffer[comm_start + 3..comm_start + 8], &[0u8; 5]);

        // Total buffer size: 8 + 4 + 4 + 8 = 24 bytes
        assert_eq!(buffer.len(), 24);
        assert_eq!(comm.header.size as usize, buffer.len());
    }

    #[test]
    fn test_sample_event_basic() {
        let sample = SampleEvent::new(
            PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD,
            0,
            0x7f0000001000,
            1234,
            5678,
            1000,
            None,
            None,
            1024,
        );

        assert_eq!(sample.header.type_, PERF_RECORD_SAMPLE);
        assert_eq!(sample.ip, 0x7f0000001000);
        assert_eq!(sample.pid, 1234);
        assert_eq!(sample.tid, 5678);
        assert_eq!(sample.period, 1000);

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        assert_eq!(buffer[0], 9);
        assert_eq!(buffer[1], 0);
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);

        let ip = u64::from_le_bytes([
            buffer[8], buffer[9], buffer[10], buffer[11], buffer[12], buffer[13], buffer[14],
            buffer[15],
        ]);
        assert_eq!(ip, 0x7f0000001000);

        let pid = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
        let tid = u32::from_le_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]);
        assert_eq!(pid, 1234);
        assert_eq!(tid, 5678);

        let time = u64::from_le_bytes([
            buffer[24], buffer[25], buffer[26], buffer[27], buffer[28], buffer[29], buffer[30],
            buffer[31],
        ]);
        assert_eq!(time, 0);

        let period = u64::from_le_bytes([
            buffer[32], buffer[33], buffer[34], buffer[35], buffer[36], buffer[37], buffer[38],
            buffer[39],
        ]);
        assert_eq!(period, 1000);
    }

    #[test]
    fn test_sample_event_with_time() {
        let sample = SampleEvent::new(
            PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD,
            1234567890123456,
            0x7f0000001000,
            1234,
            5678,
            1000,
            None,
            None,
            1024,
        );

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        let time = u64::from_le_bytes([
            buffer[24], buffer[25], buffer[26], buffer[27], buffer[28], buffer[29], buffer[30],
            buffer[31],
        ]);
        assert_eq!(time, 1234567890123456);
    }

    #[test]
    fn test_sample_event_with_callchain() {
        let callchain = vec![0x7f0000001000, 0x7f0000002000, 0x7f0000003000];
        let sample = SampleEvent::new(
            PERF_SAMPLE_IP
                | PERF_SAMPLE_TID
                | PERF_SAMPLE_TIME
                | PERF_SAMPLE_PERIOD
                | PERF_SAMPLE_CALLCHAIN,
            0,
            0x7f0000001000,
            0,
            0,
            0,
            Some(callchain.clone()),
            None,
            1024,
        );

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        let ip = u64::from_le_bytes([
            buffer[8], buffer[9], buffer[10], buffer[11], buffer[12], buffer[13], buffer[14],
            buffer[15],
        ]);
        assert_eq!(ip, 0x7f0000001000);

        let pid = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
        let tid = u32::from_le_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]);
        assert_eq!(pid, 0);
        assert_eq!(tid, 0);

        let time = u64::from_le_bytes([
            buffer[24], buffer[25], buffer[26], buffer[27], buffer[28], buffer[29], buffer[30],
            buffer[31],
        ]);
        assert_eq!(time, 0);

        // PERIOD (bit 8) comes BEFORE CALLCHAIN (bit 5) in kernel format
        let period = u64::from_le_bytes([
            buffer[32], buffer[33], buffer[34], buffer[35], buffer[36], buffer[37], buffer[38],
            buffer[39],
        ]);
        assert_eq!(period, 0);

        // CALLCHAIN comes after PERIOD
        let nr = u64::from_le_bytes([
            buffer[40], buffer[41], buffer[42], buffer[43], buffer[44], buffer[45], buffer[46],
            buffer[47],
        ]);
        assert_eq!(nr, 3);

        for i in 0..3 {
            let offset = 48 + (i * 8);
            let addr = u64::from_le_bytes([
                buffer[offset],
                buffer[offset + 1],
                buffer[offset + 2],
                buffer[offset + 3],
                buffer[offset + 4],
                buffer[offset + 5],
                buffer[offset + 6],
                buffer[offset + 7],
            ]);
            assert_eq!(addr, callchain[i]);
        }
    }

    #[test]
    fn test_sample_event_size_calculation() {
        let sample = SampleEvent::new(
            PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD,
            0,
            0,
            0,
            0,
            1000,
            None,
            None,
            1024,
        );

        let size = sample.calculate_size();
        assert_eq!(size, 40);

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();
        assert_eq!(buffer.len() as u16, size);
    }

    #[test]
    fn test_writer_with_events() {
        let mut buffer = Cursor::new(Vec::new());

        let sample_type = PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD;
        let attr = PerfEventAttr::new(0, 0, sample_type);
        let ids = vec![vec![1u64]];

        let mut writer = PerfDataWriter::new(&mut buffer);
        writer.initialize(&[attr], &ids).unwrap();

        let comm = CommEvent::new(1234, 1234, "test-program".to_string());
        writer.write_comm(&comm).unwrap();

        let mmap = MmapEvent::new(1234, 1234, 0x400000, 0x1000, 0, "/usr/bin/test".to_string());
        writer.write_mmap(&mmap).unwrap();

        let sample = SampleEvent::new(
            PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD,
            0,
            0x400123,
            1234,
            1234,
            1000,
            None,
            None,
            1024,
        );
        writer.write_sample(&sample).unwrap();

        writer.write_finished_round().unwrap();
        writer.align_to_8_bytes().unwrap();
        writer.finalize().unwrap();

        let data = buffer.into_inner();
        assert_eq!(&data[0..8], b"PERFILE2");
        assert!(data.len() > 200);

        #[cfg(feature = "evidence")]
        {
            let _ = std::fs::write(".sisyphus/evidence/test-events.perf.data", &data);
        }
    }

    #[test]
    fn test_finished_round_read_write() {
        let event = FinishedRoundEvent::new();
        let mut buffer = Vec::new();
        event.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_event = FinishedRoundEvent::read_from(&mut cursor).unwrap();

        assert_eq!(read_event.header.type_, PERF_RECORD_FINISHED_ROUND);
        assert_eq!(read_event.header.size, PERF_FINISHED_ROUND_SIZE);
    }

    #[test]
    fn test_mmap_event_read_write() {
        let mmap = MmapEvent::new(
            1234,
            5678,
            0x7f0000000000,
            0x1000,
            0,
            "/usr/bin/ls".to_string(),
        );

        let mut buffer = Vec::new();
        mmap.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_mmap = MmapEvent::read_from(&mut cursor).unwrap();

        assert_eq!(read_mmap.header.type_, PERF_RECORD_MMAP);
        assert_eq!(read_mmap.pid, 1234);
        assert_eq!(read_mmap.tid, 5678);
        assert_eq!(read_mmap.addr, 0x7f0000000000);
        assert_eq!(read_mmap.len, 0x1000);
        assert_eq!(read_mmap.pgoff, 0);
        assert_eq!(read_mmap.filename, "/usr/bin/ls");
    }

    #[test]
    fn test_mmap_event_short_filename() {
        let mmap = MmapEvent::new(1234, 5678, 0, 0x1000, 0, "ls".to_string());

        let mut buffer = Vec::new();
        mmap.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_mmap = MmapEvent::read_from(&mut cursor).unwrap();

        assert_eq!(read_mmap.filename, "ls");
    }

    #[test]
    fn test_comm_event_read_write() {
        let comm = CommEvent::new(1234, 5678, "test-program".to_string());

        let mut buffer = Vec::new();
        comm.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_comm = CommEvent::read_from(&mut cursor).unwrap();

        assert_eq!(read_comm.header.type_, PERF_RECORD_COMM);
        assert_eq!(read_comm.pid, 1234);
        assert_eq!(read_comm.tid, 5678);
        assert_eq!(read_comm.comm, "test-program");
    }

    #[test]
    fn test_comm_event_short() {
        let comm = CommEvent::new(1234, 5678, "sh".to_string());

        let mut buffer = Vec::new();
        comm.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_comm = CommEvent::read_from(&mut cursor).unwrap();

        assert_eq!(read_comm.comm, "sh");
    }

    #[test]
    fn test_sample_event_read_write_basic() {
        let sample = SampleEvent::new(
            PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD,
            0,
            0x7f0000001000,
            1234,
            5678,
            1000,
            None,
            None,
            1024,
        );

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let sample_type = PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD;
        let read_sample = SampleEvent::read_from(&mut cursor, sample_type).unwrap();

        assert_eq!(read_sample.header.type_, PERF_RECORD_SAMPLE);
        assert_eq!(read_sample.ip, 0x7f0000001000);
        assert_eq!(read_sample.pid, 1234);
        assert_eq!(read_sample.tid, 5678);
        assert_eq!(read_sample.period, 1000);
        assert!(read_sample.callchain.is_none());
    }

    #[test]
    fn test_sample_event_read_write_with_callchain() {
        let callchain = vec![0x7f0000001000, 0x7f0000002000, 0x7f0000003000];
        let sample = SampleEvent::new(
            PERF_SAMPLE_IP
                | PERF_SAMPLE_TID
                | PERF_SAMPLE_TIME
                | PERF_SAMPLE_PERIOD
                | PERF_SAMPLE_CALLCHAIN,
            0,
            0x7f0000001000,
            0,
            0,
            0,
            Some(callchain.clone()),
            None,
            1024,
        );

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let sample_type = PERF_SAMPLE_IP
            | PERF_SAMPLE_TID
            | PERF_SAMPLE_TIME
            | PERF_SAMPLE_PERIOD
            | PERF_SAMPLE_CALLCHAIN;
        let read_sample = SampleEvent::read_from(&mut cursor, sample_type).unwrap();

        assert_eq!(read_sample.ip, 0x7f0000001000);
        assert!(read_sample.callchain.is_some());
        let read_callchain = read_sample.callchain.unwrap();
        assert_eq!(read_callchain.len(), 3);
        assert_eq!(read_callchain, callchain);
    }

    #[test]
    fn test_sample_event_variable_fields() {
        let sample_type_ip_only = PERF_SAMPLE_IP;
        let sample = SampleEvent::new(
            sample_type_ip_only,
            0,
            0x7f0000001000,
            0,
            0,
            0,
            None,
            None,
            1024,
        );

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let read_sample = SampleEvent::read_from(&mut cursor, sample_type_ip_only).unwrap();

        assert_eq!(read_sample.ip, 0x7f0000001000);
        assert_eq!(read_sample.pid, 0);
        assert_eq!(read_sample.tid, 0);
        assert_eq!(read_sample.time, 0);
        assert_eq!(read_sample.period, 0);
    }

    #[test]
    fn test_read_null_terminated_padded_string() {
        let test_strings = vec!["test", "a", "longer-string-name", ""];
        let alignment = 8;

        for s in test_strings {
            let mut buffer = Vec::new();
            write_null_terminated_padded_string(&mut buffer, s, alignment).unwrap();

            let mut cursor = Cursor::new(buffer);
            let read_s = read_null_terminated_padded_string(&mut cursor, alignment).unwrap();

            assert_eq!(read_s, s, "Failed for string: '{}'", s);
        }
    }
}

// Reader Implementation

/// Reader for Linux perf.data files
///
/// This reader parses files compatible with the Linux perf tool.
pub struct PerfDataReader<R: Read + Seek> {
    reader: R,
    header: PerfFileHeader,
    attrs: Vec<PerfEventAttr>,
    event_ids: Vec<Vec<u64>>,
    data_offset: u64,
    data_size: u64,
}

impl<R: Read + Seek> PerfDataReader<R> {
    /// Create a new reader from a generic reader that supports Read and Seek
    pub fn from_reader(reader: R) -> io::Result<Self> {
        let mut data_reader = Self {
            reader,
            header: PerfFileHeader::default(),
            attrs: Vec::new(),
            event_ids: Vec::new(),
            data_offset: 0,
            data_size: 0,
        };

        data_reader.parse_header()?;
        data_reader.parse_attributes()?;

        Ok(data_reader)
    }

    /// Parse the file header from the beginning of the file
    fn parse_header(&mut self) -> io::Result<()> {
        use byteorder::{LittleEndian, ReadBytesExt};

        self.reader.seek(SeekFrom::Start(0))?;

        let mut magic_bytes = [0u8; 8];
        self.reader.read_exact(&mut magic_bytes)?;

        if &magic_bytes != b"PERFILE2" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid magic number: expected 'PERFILE2', got '{:?}'",
                    magic_bytes
                ),
            ));
        }

        let magic = u64::from_le_bytes(magic_bytes);
        let size = self.reader.read_u64::<LittleEndian>()?;
        let attr_size = self.reader.read_u64::<LittleEndian>()?;

        let attrs = PerfFileSection {
            offset: self.reader.read_u64::<LittleEndian>()?,
            size: self.reader.read_u64::<LittleEndian>()?,
        };

        let data = PerfFileSection {
            offset: self.reader.read_u64::<LittleEndian>()?,
            size: self.reader.read_u64::<LittleEndian>()?,
        };

        let event_types = PerfFileSection {
            offset: self.reader.read_u64::<LittleEndian>()?,
            size: self.reader.read_u64::<LittleEndian>()?,
        };

        let flags = self.reader.read_u64::<LittleEndian>()?;
        let mut flags1 = [0u64; 3];
        for flag in flags1.iter_mut() {
            *flag = self.reader.read_u64::<LittleEndian>()?;
        }

        self.header = PerfFileHeader {
            magic,
            size,
            attr_size,
            attrs,
            data,
            event_types,
            flags,
            flags1,
        };

        self.header.validate().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Invalid header: {}", e))
        })?;

        self.data_offset = self.header.data.offset;
        self.data_size = self.header.data.size;

        Ok(())
    }

    /// Parse the attribute section and event IDs
    fn parse_attributes(&mut self) -> io::Result<()> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let attrs_offset = self.header.attrs.offset;
        let attrs_size = self.header.attrs.size;
        // attr_size from header is the total entry size (attr + ids_offset + ids_size)
        let entry_size = self.header.attr_size as usize;

        if attrs_size == 0 {
            return Ok(());
        }

        self.reader.seek(SeekFrom::Start(attrs_offset))?;

        let attrs_end = attrs_offset + attrs_size;
        while self.reader.stream_position()? < attrs_end {
            // Read the attr structure (variable size based on attr.size field)
            let attr = PerfEventAttr::read_from(&mut self.reader)?;

            self.attrs.push(attr);

            // Read ids_offset and ids_size (these are part of the attr entry)
            let ids_offset = self.reader.read_u64::<LittleEndian>()?;
            let ids_size = self.reader.read_u64::<LittleEndian>()?;

            let mut ids = Vec::new();
            if ids_size > 0 {
                let current_pos = self.reader.stream_position()?;
                self.reader.seek(SeekFrom::Start(ids_offset))?;

                let num_ids = ids_size / 8;
                for _ in 0..num_ids {
                    ids.push(self.reader.read_u64::<LittleEndian>()?);
                }

                self.reader.seek(SeekFrom::Start(current_pos))?;
            }

            self.event_ids.push(ids);

            // Skip any remaining bytes to align to entry_size boundary
            // This handles cases where the file has padding or future extensions
            let bytes_consumed = PERF_ATTR_SIZE_VER8 as usize + 16; // attr + ids_offset + ids_size
            if bytes_consumed < entry_size {
                let remaining = entry_size - bytes_consumed;
                let mut skip = vec![0u8; remaining];
                self.reader.read_exact(&mut skip)?;
            }
        }

        Ok(())
    }

    /// Get a reference to the file header
    pub fn header(&self) -> &PerfFileHeader {
        &self.header
    }

    /// Get a reference to the parsed attributes
    pub fn attrs(&self) -> &[PerfEventAttr] {
        &self.attrs
    }

    /// Get a reference to the event IDs
    pub fn event_ids(&self) -> &[Vec<u64>] {
        &self.event_ids
    }

    /// Get the data section offset
    pub fn data_offset(&self) -> u64 {
        self.data_offset
    }

    /// Get the data section size
    pub fn data_size(&self) -> u64 {
        self.data_size
    }

    /// Seek to the beginning of the data section
    pub fn seek_to_data(&mut self) -> io::Result<()> {
        self.reader.seek(SeekFrom::Start(self.data_offset))?;
        Ok(())
    }

    /// Get a mutable reference to the underlying reader
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }
}

impl PerfDataReader<File> {
    /// Create a reader from a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }
}

// Event types for streaming from perf.data files

/// Enum representing all possible perf event types
#[derive(Debug, Clone)]
pub enum Event {
    /// PERF_RECORD_MMAP: Memory mapping event
    Mmap(MmapEvent),
    /// PERF_RECORD_LOST: Lost events
    Lost(PerfEventHeader),
    /// PERF_RECORD_COMM: Command name event
    Comm(CommEvent),
    /// PERF_RECORD_EXIT: Exit event
    Exit(PerfEventHeader),
    /// PERF_RECORD_THROTTLE: Throttling event
    Throttle(PerfEventHeader),
    /// PERF_RECORD_UNTHROTTLE: Unthrottling event
    Unthrottle(PerfEventHeader),
    /// PERF_RECORD_FORK: Fork event
    Fork(PerfEventHeader),
    /// PERF_RECORD_READ: Read event
    Read(PerfEventHeader),
    /// PERF_RECORD_SAMPLE: Sample event
    Sample(SampleEvent),
    /// PERF_RECORD_MMAP2: Extended memory mapping event
    Mmap2(PerfEventHeader),
    /// PERF_RECORD_AUX: Auxiliary data
    Aux(PerfEventHeader),
    /// PERF_RECORD_ITRACE_START: Instruction trace start
    ItraceStart(PerfEventHeader),
    /// PERF_RECORD_LOST_SAMPLES: Lost samples
    LostSamples(PerfEventHeader),
    /// PERF_RECORD_SWITCH: Context switch event
    Switch(PerfEventHeader),
    /// PERF_RECORD_SWITCH_CPU_WIDE: CPU-wide context switch
    SwitchCpuWide(PerfEventHeader),
    /// PERF_RECORD_NAMESPACES: Namespaces event
    Namespaces(PerfEventHeader),
    /// PERF_RECORD_KSYMBOL: Kernel symbol event
    Ksymbol(PerfEventHeader),
    /// PERF_RECORD_BPF_EVENT: BPF event
    BpfEvent(PerfEventHeader),
    /// PERF_RECORD_CGROUP: Cgroup event
    Cgroup(PerfEventHeader),
    /// PERF_RECORD_TEXT_POKE: Text poke event
    TextPoke(PerfEventHeader),
    /// PERF_RECORD_AUX_OUTPUT_HW_ID: AUX output hardware ID
    AuxOutputHwId(PerfEventHeader),
    /// PERF_RECORD_CALLCHAIN_DEFERRED: Deferred callchain
    CallchainDeferred(PerfEventHeader),
    /// PERF_RECORD_FINISHED_ROUND: Round marker (batch boundary)
    FinishedRound(FinishedRoundEvent),
    /// PERF_RECORD_ID_INDEX: ID index
    IdIndex(PerfEventHeader),
    /// PERF_RECORD_AUXTRACE_INFO: AUX trace info
    AuxtraceInfo(PerfEventHeader),
    /// PERF_RECORD_AUXTRACE: AUX trace data
    Auxtrace(PerfEventHeader),
    /// PERF_RECORD_AUXTRACE_ERROR: AUX trace error
    AuxtraceError(PerfEventHeader),
    /// PERF_RECORD_HEADER_FEATURE: Header feature
    HeaderFeature(PerfEventHeader),
    /// PERF_RECORD_COMPRESSED: Compressed data
    Compressed(PerfEventHeader),
    /// PERF_RECORD_FINISHED_INIT: Finished initialization
    FinishedInit(PerfEventHeader),
    /// PERF_RECORD_COMPRESSED2: Compressed data (format 2)
    Compressed2(PerfEventHeader),
    /// Unknown event type
    Unknown(PerfEventHeader),
}

impl<R: Read + Seek> PerfDataReader<R> {
    /// Create an iterator over all events in the data section
    ///
    /// Returns an iterator that lazily reads events from the perf.data file.
    /// Events are read one at a time, providing memory-efficient streaming.
    pub fn event_iter(&mut self) -> io::Result<EventIterator<R>> {
        let sample_type = self.attrs.first().map(|a| a.sample_type).unwrap_or(0);
        EventIterator::new(
            &mut self.reader,
            self.data_offset,
            self.data_size,
            sample_type,
        )
    }

    /// Create a filtered iterator over events of a specific type
    ///
    /// # Arguments
    ///
    /// * `event_type` - The PERF_RECORD_* constant to filter by (e.g., PERF_RECORD_SAMPLE)
    pub fn event_filter(&mut self, event_type: u32) -> io::Result<EventIterator<R>> {
        let sample_type = self.attrs.first().map(|a| a.sample_type).unwrap_or(0);
        EventIterator::with_filter(
            &mut self.reader,
            self.data_offset,
            self.data_size,
            event_type,
            sample_type,
        )
    }

    /// Read all events from the data section into a vector
    ///
    /// This method reads all events from the perf.data file and returns them as a Vec<Event>.
    /// Note: This loads all events into memory, so use with caution for large files.
    pub fn read_all_events(&mut self) -> io::Result<Vec<Event>> {
        let mut events = Vec::new();
        let iter = self.event_iter()?;

        for event in iter {
            events.push(event?);
        }

        Ok(events)
    }
}

/// Iterator for streaming events from perf.data files
///
/// Provides lazy parsing of events from the data section of a perf.data file.
/// Events are read on-demand, making it memory-efficient for large files.
pub struct EventIterator<'a, R: Read + Seek> {
    reader: &'a mut R,
    data_offset: u64,
    data_size: u64,
    current_offset: u64,
    filter: Option<u32>,
    sample_type: u64,
}

impl<'a, R: Read + Seek> EventIterator<'a, R> {
    /// Create a new event iterator
    ///
    /// # Arguments
    ///
    /// * `reader` - Mutable reference to the reader
    /// * `data_offset` - Starting offset of the data section
    /// * `data_size` - Size of the data section
    /// * `sample_type` - Sample type bitmask from PerfEventAttr for parsing SAMPLE events
    fn new(
        reader: &'a mut R,
        data_offset: u64,
        data_size: u64,
        sample_type: u64,
    ) -> io::Result<Self> {
        let mut iter = Self {
            reader,
            data_offset,
            data_size,
            current_offset: data_offset,
            filter: None,
            sample_type,
        };
        iter.seek_to_start()?;
        Ok(iter)
    }

    /// Create a filtered event iterator
    ///
    /// # Arguments
    ///
    /// * `reader` - Mutable reference to the reader
    /// * `data_offset` - Starting offset of the data section
    /// * `data_size` - Size of the data section
    /// * `event_type` - Event type to filter by (PERF_RECORD_* constant)
    /// * `sample_type` - Sample type bitmask from PerfEventAttr for parsing SAMPLE events
    fn with_filter(
        reader: &'a mut R,
        data_offset: u64,
        data_size: u64,
        event_type: u32,
        sample_type: u64,
    ) -> io::Result<Self> {
        let mut iter = Self {
            reader,
            data_offset,
            data_size,
            current_offset: data_offset,
            filter: Some(event_type),
            sample_type,
        };
        iter.seek_to_start()?;
        Ok(iter)
    }

    fn seek_to_start(&mut self) -> io::Result<()> {
        self.reader.seek(SeekFrom::Start(self.data_offset))?;
        self.current_offset = self.data_offset;
        Ok(())
    }

    fn align_to_8_bytes(&mut self) -> io::Result<()> {
        let current_pos = self.reader.stream_position()?;
        let alignment = 8;
        let padding = (alignment - (current_pos as usize % alignment)) % alignment;
        if padding > 0 {
            let mut padding_buf = vec![0u8; padding];
            self.reader.read_exact(&mut padding_buf)?;
        }
        Ok(())
    }
}

impl<'a, R: Read + Seek> Iterator for EventIterator<'a, R> {
    type Item = io::Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_offset >= self.data_offset + self.data_size {
            return None;
        }

        let header_result = PerfEventHeader::read_from(self.reader);

        let header = match header_result {
            Ok(h) => h,
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return None;
                }
                return Some(Err(e));
            }
        };

        let current_pos = match self.reader.stream_position() {
            Ok(pos) => pos,
            Err(e) => return Some(Err(e)),
        };

        if current_pos > self.data_offset + self.data_size {
            return None;
        }

        self.current_offset = current_pos;

        // Sanity check: header size must be at least the header itself (8 bytes) and not too large
        if header.size < PERF_EVENT_HEADER_SIZE || header.size > 10000 {
            return None;
        }

        self.process_event(header)
    }
}

impl<'a, R: Read + Seek> EventIterator<'a, R> {
    fn process_event(&mut self, header: PerfEventHeader) -> Option<io::Result<Event>> {
        let event_result = match header.type_ {
            PERF_RECORD_MMAP => MmapEvent::read_from_header(self.reader, header).map(Event::Mmap),
            PERF_RECORD_COMM => CommEvent::read_from_header(self.reader, header).map(Event::Comm),
            PERF_RECORD_SAMPLE => {
                SampleEvent::read_from_header(self.reader, header, self.sample_type)
                    .map(Event::Sample)
            }
            PERF_RECORD_FINISHED_ROUND => {
                FinishedRoundEvent::read_from_header(self.reader, header).map(Event::FinishedRound)
            }
            _ => {
                let payload_size =
                    (header.size as usize).saturating_sub(PERF_EVENT_HEADER_SIZE as usize);
                if payload_size > 0 {
                    let mut payload = vec![0u8; payload_size];
                    if let Err(e) = self.reader.read_exact(&mut payload) {
                        return Some(Err(e));
                    }
                }

                Ok(match header.type_ {
                    PERF_RECORD_LOST => Event::Lost(header),
                    PERF_RECORD_EXIT => Event::Exit(header),
                    PERF_RECORD_THROTTLE => Event::Throttle(header),
                    PERF_RECORD_UNTHROTTLE => Event::Unthrottle(header),
                    PERF_RECORD_FORK => Event::Fork(header),
                    PERF_RECORD_READ => Event::Read(header),
                    PERF_RECORD_MMAP2 => Event::Mmap2(header),
                    PERF_RECORD_AUX => Event::Aux(header),
                    PERF_RECORD_ITRACE_START => Event::ItraceStart(header),
                    PERF_RECORD_LOST_SAMPLES => Event::LostSamples(header),
                    PERF_RECORD_SWITCH => Event::Switch(header),
                    PERF_RECORD_SWITCH_CPU_WIDE => Event::SwitchCpuWide(header),
                    PERF_RECORD_NAMESPACES => Event::Namespaces(header),
                    PERF_RECORD_KSYMBOL => Event::Ksymbol(header),
                    PERF_RECORD_BPF_EVENT => Event::BpfEvent(header),
                    PERF_RECORD_CGROUP => Event::Cgroup(header),
                    PERF_RECORD_TEXT_POKE => Event::TextPoke(header),
                    PERF_RECORD_AUX_OUTPUT_HW_ID => Event::AuxOutputHwId(header),
                    PERF_RECORD_CALLCHAIN_DEFERRED => Event::CallchainDeferred(header),
                    PERF_RECORD_ID_INDEX => Event::IdIndex(header),
                    PERF_RECORD_AUXTRACE_INFO => Event::AuxtraceInfo(header),
                    PERF_RECORD_AUXTRACE => Event::Auxtrace(header),
                    PERF_RECORD_AUXTRACE_ERROR => Event::AuxtraceError(header),
                    PERF_RECORD_HEADER_FEATURE => Event::HeaderFeature(header),
                    PERF_RECORD_COMPRESSED => Event::Compressed(header),
                    PERF_RECORD_FINISHED_INIT => Event::FinishedInit(header),
                    PERF_RECORD_COMPRESSED2 => Event::Compressed2(header),
                    _ => Event::Unknown(header),
                })
            }
        };

        match self.reader.stream_position() {
            Ok(pos) => self.current_offset = pos,
            Err(e) => return Some(Err(e)),
        }

        match event_result {
            Ok(event) => {
                if let Some(filter_type) = self.filter {
                    let matches_filter = match &event {
                        Event::FinishedRound(_) => filter_type == PERF_RECORD_FINISHED_ROUND,
                        Event::Mmap(_) => filter_type == PERF_RECORD_MMAP,
                        Event::Comm(_) => filter_type == PERF_RECORD_COMM,
                        Event::Sample(_) => filter_type == PERF_RECORD_SAMPLE,
                        Event::Lost(_) => filter_type == PERF_RECORD_LOST,
                        Event::Exit(_) => filter_type == PERF_RECORD_EXIT,
                        Event::Throttle(_) => filter_type == PERF_RECORD_THROTTLE,
                        Event::Unthrottle(_) => filter_type == PERF_RECORD_UNTHROTTLE,
                        Event::Fork(_) => filter_type == PERF_RECORD_FORK,
                        Event::Read(_) => filter_type == PERF_RECORD_READ,
                        Event::Mmap2(_) => filter_type == PERF_RECORD_MMAP2,
                        Event::Aux(_) => filter_type == PERF_RECORD_AUX,
                        Event::ItraceStart(_) => filter_type == PERF_RECORD_ITRACE_START,
                        Event::LostSamples(_) => filter_type == PERF_RECORD_LOST_SAMPLES,
                        Event::Switch(_) => filter_type == PERF_RECORD_SWITCH,
                        Event::SwitchCpuWide(_) => filter_type == PERF_RECORD_SWITCH_CPU_WIDE,
                        Event::Namespaces(_) => filter_type == PERF_RECORD_NAMESPACES,
                        Event::Ksymbol(_) => filter_type == PERF_RECORD_KSYMBOL,
                        Event::BpfEvent(_) => filter_type == PERF_RECORD_BPF_EVENT,
                        Event::Cgroup(_) => filter_type == PERF_RECORD_CGROUP,
                        Event::TextPoke(_) => filter_type == PERF_RECORD_TEXT_POKE,
                        Event::AuxOutputHwId(_) => filter_type == PERF_RECORD_AUX_OUTPUT_HW_ID,
                        Event::CallchainDeferred(_) => {
                            filter_type == PERF_RECORD_CALLCHAIN_DEFERRED
                        }
                        Event::IdIndex(_) => filter_type == PERF_RECORD_ID_INDEX,
                        Event::AuxtraceInfo(_) => filter_type == PERF_RECORD_AUXTRACE_INFO,
                        Event::Auxtrace(_) => filter_type == PERF_RECORD_AUXTRACE,
                        Event::AuxtraceError(_) => filter_type == PERF_RECORD_AUXTRACE_ERROR,
                        Event::HeaderFeature(_) => filter_type == PERF_RECORD_HEADER_FEATURE,
                        Event::Compressed(_) => filter_type == PERF_RECORD_COMPRESSED,
                        Event::FinishedInit(_) => filter_type == PERF_RECORD_FINISHED_INIT,
                        Event::Compressed2(_) => filter_type == PERF_RECORD_COMPRESSED2,
                        Event::Unknown(_) => false,
                    };

                    if !matches_filter {
                        return self.next();
                    }
                }

                if matches!(&event, Event::FinishedRound(_)) {
                    return self.next();
                }

                Some(Ok(event))
            }
            Err(e) => Some(Err(e)),
        }
    }
}
