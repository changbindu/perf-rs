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

/// Magic number for perf.data files (little-endian "PERFILE2")
pub const PERF_FILE_MAGIC: u64 = 0x50455246494c4532;

/// Size of the perf_file_header structure (104 bytes)
pub const PERF_FILE_HEADER_SIZE: u64 = 104;

/// Size of perf_event_attr in kernel 6.17
pub const PERF_ATTR_SIZE_VER8: u32 = 136;

/// Event header size (6 bytes: type + misc + size)
pub const PERF_EVENT_HEADER_SIZE: u16 = 6;

/// FINISHED_ROUND event size (6 bytes - just the event header)
pub const PERF_FINISHED_ROUND_SIZE: u16 = 6;

// Perf record types
pub const PERF_RECORD_MMAP: u16 = 1;
pub const PERF_RECORD_LOST: u16 = 2;
pub const PERF_RECORD_COMM: u16 = 3;
pub const PERF_RECORD_EXIT: u16 = 4;
pub const PERF_RECORD_THROTTLE: u16 = 5;
pub const PERF_RECORD_UNTHROTTLE: u16 = 6;
pub const PERF_RECORD_FORK: u16 = 7;
pub const PERF_RECORD_READ: u16 = 8;
pub const PERF_RECORD_SAMPLE: u16 = 9;
pub const PERF_RECORD_MMAP2: u16 = 10;
pub const PERF_RECORD_AUX: u16 = 11;
pub const PERF_RECORD_ITRACE_START: u16 = 12;
pub const PERF_RECORD_LOST_SAMPLES: u16 = 13;
pub const PERF_RECORD_SWITCH: u16 = 14;
pub const PERF_RECORD_SWITCH_CPU_WIDE: u16 = 15;
pub const PERF_RECORD_NAMESPACES: u16 = 16;
pub const PERF_RECORD_KSYMBOL: u16 = 17;
pub const PERF_RECORD_BPF_EVENT: u16 = 18;
pub const PERF_RECORD_CGROUP: u16 = 19;
pub const PERF_RECORD_TEXT_POKE: u16 = 20;
pub const PERF_RECORD_AUX_OUTPUT_HW_ID: u16 = 21;
pub const PERF_RECORD_CALLCHAIN_DEFERRED: u16 = 22;
pub const PERF_RECORD_HEADER_ATTR: u16 = 64;
pub const PERF_RECORD_HEADER_EVENT_TYPE: u16 = 65;
pub const PERF_RECORD_HEADER_TRACING_DATA: u16 = 66;
pub const PERF_RECORD_HEADER_BUILD_ID: u16 = 67;
pub const PERF_RECORD_FINISHED_ROUND: u16 = 68;
pub const PERF_RECORD_ID_INDEX: u16 = 69;
pub const PERF_RECORD_AUXTRACE_INFO: u16 = 70;
pub const PERF_RECORD_AUXTRACE: u16 = 71;
pub const PERF_RECORD_AUXTRACE_ERROR: u16 = 72;
pub const PERF_RECORD_HEADER_FEATURE: u16 = 80;
pub const PERF_RECORD_COMPRESSED: u16 = 81;
pub const PERF_RECORD_FINISHED_INIT: u16 = 82;
pub const PERF_RECORD_COMPRESSED2: u16 = 83;

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
    /// Temporary: sample_count for backward compatibility (not in Linux perf format)
    #[allow(dead_code)]
    pub sample_count: u64,
    /// Temporary: mmap_count for backward compatibility (not in Linux perf format)
    #[allow(dead_code)]
    pub mmap_count: u64,
    /// Temporary: comm_count for backward compatibility (not in Linux perf format)
    #[allow(dead_code)]
    pub comm_count: u64,
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
            sample_count: 0,
            mmap_count: 0,
            comm_count: 0,
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
            magic: u64::from_be_bytes(magic_bytes),
            size,
            attr_size,
            attrs,
            data,
            event_types,
            flags,
            flags1,
            sample_count: 0,
            mmap_count: 0,
            comm_count: 0,
        })
    }

    pub fn validate(&self) -> Result<(), PerfError> {
        if self.magic != PERF_FILE_MAGIC {
            return Err(PerfError::InvalidMagic {
                expected: "PERFILE2".to_string(),
                actual: format!("{:016x}", self.magic),
            });
        }
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
    /// Config 4
    pub config4: u64,
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
            config4: 0,
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
        writer.write_u64::<LittleEndian>(self.config4)?;

        Ok(())
    }
}

// Event Header and Records

/// Common header for all event records (6 bytes, padded to 8-byte boundary)
#[derive(Debug, Clone, Copy)]
pub struct PerfEventHeader {
    /// Event type (PERF_RECORD_*)
    pub type_: u16,
    /// Additional flags
    pub misc: u16,
    /// Total size of the event record including header
    pub size: u16,
}

impl PerfEventHeader {
    pub fn new(type_: u16, size: u16) -> Self {
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
        writer.write_u16::<LittleEndian>(self.type_)?;
        writer.write_u16::<LittleEndian>(self.misc)?;
        writer.write_u16::<LittleEndian>(self.size)?;
        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            type_: reader.read_u16::<LittleEndian>()?,
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
        // Calculate size: header (6) + pid (4) + tid (4) + addr (8) + len (8) + pgoff (8) + filename (with null and padding)
        let filename_bytes_len = filename.len() + 1; // +1 for null terminator
        let padded_len = ((filename_bytes_len + 7) / 8) * 8; // Pad to 8 bytes
        let total_size = 6 + 4 + 4 + 8 + 8 + 8 + padded_len;

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

        // Write null-terminated filename with padding to 8 bytes
        write_null_terminated_padded_string(writer, &self.filename, 8)?;

        Ok(())
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
        // Calculate size: header (6) + pid (4) + tid (4) + comm (with null and padding)
        let comm_bytes_len = comm.len() + 1; // +1 for null terminator
        let padded_len = ((comm_bytes_len + 7) / 8) * 8; // Pad to 8 bytes
        let total_size = 6 + 4 + 4 + padded_len;

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

        // Write null-terminated comm with padding to 8 bytes
        write_null_terminated_padded_string(writer, &self.comm, 8)?;

        Ok(())
    }
}

/// PERF_RECORD_SAMPLE event (type 9)
///
/// Records a sample event with variable fields based on sample_type.
#[derive(Debug, Clone)]
pub struct SampleEvent {
    pub header: PerfEventHeader,
    pub sample_type: u64,
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
        time: u64,
        ip: u64,
        pid: u32,
        tid: u32,
        period: u64,
        callchain: Option<Vec<u64>>,
        cpu: Option<u32>,
    ) -> Self {
        let sample_type = PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_PERIOD;
        let has_callchain = callchain.as_ref().map_or(false, |cc| !cc.is_empty());

        // Calculate size based on sample_type fields
        let mut size = 6u16; // Header size

        if sample_type & PERF_SAMPLE_IP != 0 {
            size += 8;
        }
        if sample_type & PERF_SAMPLE_TID != 0 {
            size += 8;
        }
        if sample_type & PERF_SAMPLE_TIME != 0 {
            size += 8;
        }
        if sample_type & PERF_SAMPLE_PERIOD != 0 {
            size += 8;
        }
        if let Some(ref cc) = callchain {
            // nr (8) + nr * 8 for each IP
            size += 8 + (cc.len() as u16 * 8);
        }

        Self {
            header: PerfEventHeader::new(PERF_RECORD_SAMPLE, size),
            sample_type,
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
        let mut size = 6u16; // Header size

        if self.sample_type & PERF_SAMPLE_IP != 0 {
            size += 8;
        }
        if self.sample_type & PERF_SAMPLE_TID != 0 {
            size += 8;
        }
        if self.sample_type & PERF_SAMPLE_TIME != 0 {
            size += 8;
        }
        if self.sample_type & PERF_SAMPLE_PERIOD != 0 {
            size += 8;
        }
        if let Some(ref cc) = self.callchain {
            if !cc.is_empty() {
                // nr (u64) + nr * u64 for each IP
                size += 8 + (cc.len() as u16 * 8);
            }
        }

        size
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        // Recalculate size with actual callchain length
        let size = self.calculate_size();
        let header = PerfEventHeader::new(PERF_RECORD_SAMPLE, size);

        header.write_to(writer)?;

        // Write fields in the order they appear in the format
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

        if self.sample_type & PERF_SAMPLE_PERIOD != 0 {
            writer.write_u64::<LittleEndian>(self.period)?;
        }

        if let Some(ref cc) = self.callchain {
            if !cc.is_empty() {
                writer.write_u64::<LittleEndian>(cc.len() as u64)?;
                for ip in cc {
                    writer.write_u64::<LittleEndian>(*ip)?;
                }
            }
        }

        Ok(())
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

    /// Initialize the file header with attributes section
    ///
    /// This writes the file header and attributes section to the file.
    /// After calling this method, the writer is positioned at the data section.
    pub fn initialize(&mut self, attrs: &[PerfEventAttr], ids: &[Vec<u64>]) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        let header_start = self.writer.stream_position()?;
        self.header.write_to(&mut self.writer)?;

        let attrs_offset = header_start + PERF_FILE_HEADER_SIZE;

        let mut attrs_size = 0u64;
        for (attr, id_vec) in attrs.iter().zip(ids.iter()) {
            attr.write_to(&mut self.writer)?;
            attrs_size += PERF_ATTR_SIZE_VER8 as u64;

            let ids_offset =
                attrs_offset + attrs_size + (ids.len() as u64 * (PERF_ATTR_SIZE_VER8 as u64 + 16));

            let ids_size = id_vec.len() as u64 * 8;
            self.writer.write_u64::<LittleEndian>(ids_offset)?;
            self.writer.write_u64::<LittleEndian>(ids_size)?;
            attrs_size += 16;
        }

        for id_vec in ids {
            for id in id_vec {
                self.writer.write_u64::<LittleEndian>(*id)?;
            }
        }

        let current_pos = self.writer.stream_position()?;
        self.data_start_offset = (current_pos + 7) & !7;

        let padding = self.data_start_offset - current_pos;
        if padding > 0 {
            writer_write_padding(&mut self.writer, padding as usize)?;
        }

        self.header.attrs.offset = attrs_offset;
        self.header.attrs.size = attrs_size;
        self.header.data.offset = self.data_start_offset;
        self.header.data.size = 0;

        self.writer.seek(SeekFrom::Start(header_start))?;
        self.header.write_to(&mut self.writer)?;
        self.writer.seek(SeekFrom::Start(self.data_start_offset))?;

        self.attrs_written = true;
        Ok(())
    }

    /// Write an event header
    pub fn write_event_header(&mut self, header: &PerfEventHeader) -> io::Result<()> {
        header.write_to(&mut self.writer)?;
        self.data_size += header.size as u64;
        Ok(())
    }

    /// Write a FINISHED_ROUND event
    pub fn write_finished_round(&mut self) -> io::Result<()> {
        let event = FinishedRoundEvent::new();
        event.write_to(&mut self.writer)?;
        self.data_size += PERF_FINISHED_ROUND_SIZE as u64;
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

    /// Finalize the file and update header (backward compatibility method)
    pub fn finalize_with_header_update(mut self) -> io::Result<()> {
        self.header.data.size = self.data_size;

        let header_start = 0;
        self.writer.seek(SeekFrom::Start(header_start))?;
        self.header.write_to(&mut self.writer)?;
        self.writer.flush()?;

        Ok(())
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

        // type_ (u16 little-endian): bytes 0-1
        assert_eq!(buffer[0], 68);
        assert_eq!(buffer[1], 0);

        // misc (u16 little-endian): bytes 2-3
        assert_eq!(buffer[2], 0);
        assert_eq!(buffer[3], 0);

        // size (u16 little-endian): bytes 4-5
        assert_eq!(buffer[4], 6);
        assert_eq!(buffer[5], 0);
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
        assert_eq!(buffer[1], 0); // type MSB
        assert_eq!(
            u16::from_le_bytes([buffer[4], buffer[5]]),
            buffer.len() as u16
        ); // size

        // Verify pid/tid
        let pid = u32::from_le_bytes([buffer[6], buffer[7], buffer[8], buffer[9]]);
        let tid = u32::from_le_bytes([buffer[10], buffer[11], buffer[12], buffer[13]]);
        assert_eq!(pid, 1234);
        assert_eq!(tid, 5678);

        // Verify filename with null terminator
        let filename_start = 6 + 4 + 4 + 8 + 8 + 8; // After header + pid + tid + addr + len + pgoff
        let filename_bytes = &buffer[filename_start..];
        assert_eq!(&filename_bytes[0..12], b"/usr/bin/ls\0");
    }

    #[test]
    fn test_mmap_event_alignment() {
        // Test that filename is properly padded to 8-byte alignment
        let short_filename = "ls";
        let mmap = MmapEvent::new(1234, 5678, 0, 0x1000, 0, short_filename.to_string());

        let mut buffer = Vec::new();
        mmap.write_to(&mut buffer).unwrap();

        // The filename field should be padded to 8 bytes
        // Filename starts at offset: header(6) + pid(4) + tid(4) + addr(8) + len(8) + pgoff(8) = 38
        let filename_start = 38;
        // "ls" (2 bytes) + null (1 byte) + padding (5 bytes) = 8 bytes total
        assert_eq!(&buffer[filename_start..filename_start + 2], b"ls");
        assert_eq!(buffer[filename_start + 2], 0); // null terminator
                                                   // The next 5 bytes should be padding (zeros)
        assert_eq!(&buffer[filename_start + 3..filename_start + 8], &[0u8; 5]);

        // Total buffer size
        assert_eq!(buffer.len(), 46);
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
        assert_eq!(buffer[1], 0); // type MSB

        // Verify pid/tid
        let pid = u32::from_le_bytes([buffer[6], buffer[7], buffer[8], buffer[9]]);
        let tid = u32::from_le_bytes([buffer[10], buffer[11], buffer[12], buffer[13]]);
        assert_eq!(pid, 1234);
        assert_eq!(tid, 5678);

        // Verify comm string with null terminator
        let comm_start = 6 + 4 + 4; // After header + pid + tid
        let comm_bytes = &buffer[comm_start..];
        assert_eq!(&comm_bytes[0..13], b"test-program\0");
    }

    #[test]
    fn test_comm_event_alignment() {
        // Test that comm is properly padded to 8-byte alignment
        let short_comm = "sh";
        let comm = CommEvent::new(1234, 5678, short_comm.to_string());

        let mut buffer = Vec::new();
        comm.write_to(&mut buffer).unwrap();

        // The comm field should be padded to 8 bytes
        // Comm starts at offset: header(6) + pid(4) + tid(4) = 14
        let comm_start = 14;
        // "sh" (2 bytes) + null (1 byte) + padding (5 bytes) = 8 bytes total
        assert_eq!(&buffer[comm_start..comm_start + 2], b"sh");
        assert_eq!(buffer[comm_start + 2], 0); // null terminator
                                               // The next 5 bytes should be padding (zeros)
        assert_eq!(&buffer[comm_start + 3..comm_start + 8], &[0u8; 5]);

        // Total buffer size: 6 + 4 + 4 + 8 = 22 bytes
        assert_eq!(buffer.len(), 22);
        assert_eq!(comm.header.size as usize, buffer.len());
    }

    #[test]
    fn test_sample_event_basic() {
        let sample = SampleEvent::new(0, 0x7f0000001000, 1234, 5678, 1000, None, None);

        assert_eq!(sample.header.type_, PERF_RECORD_SAMPLE);
        assert_eq!(sample.ip, 0x7f0000001000);
        assert_eq!(sample.pid, 1234);
        assert_eq!(sample.tid, 5678);
        assert_eq!(sample.period, 1000);

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        // Verify header
        assert_eq!(buffer[0], 9); // SAMPLE type
        assert_eq!(buffer[1], 0); // type MSB

        // Verify IP field
        let ip = u64::from_le_bytes([
            buffer[6], buffer[7], buffer[8], buffer[9], buffer[10], buffer[11], buffer[12],
            buffer[13],
        ]);
        assert_eq!(ip, 0x7f0000001000);

        // Verify pid/tid fields
        let pid = u32::from_le_bytes([buffer[14], buffer[15], buffer[16], buffer[17]]);
        let tid = u32::from_le_bytes([buffer[18], buffer[19], buffer[20], buffer[21]]);
        assert_eq!(pid, 1234);
        assert_eq!(tid, 5678);

        // Verify time field (always present with new signature)
        let time = u64::from_le_bytes([
            buffer[22], buffer[23], buffer[24], buffer[25], buffer[26], buffer[27], buffer[28],
            buffer[29],
        ]);
        assert_eq!(time, 0);

        // Verify period field (after time)
        let period = u64::from_le_bytes([
            buffer[30], buffer[31], buffer[32], buffer[33], buffer[34], buffer[35], buffer[36],
            buffer[37],
        ]);
        assert_eq!(period, 1000);
    }

    #[test]
    fn test_sample_event_with_time() {
        let sample = SampleEvent::new(
            1234567890123456,
            0x7f0000001000,
            1234,
            5678,
            1000,
            None,
            None,
        );

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        // Verify time field (after IP, TID)
        let time = u64::from_le_bytes([
            buffer[22], buffer[23], buffer[24], buffer[25], buffer[26], buffer[27], buffer[28],
            buffer[29],
        ]);
        assert_eq!(time, 1234567890123456);
    }

    #[test]
    fn test_sample_event_with_callchain() {
        let callchain = vec![0x7f0000001000, 0x7f0000002000, 0x7f0000003000];
        let sample = SampleEvent::new(0, 0x7f0000001000, 0, 0, 0, Some(callchain.clone()), None);

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();

        // Verify IP field
        let ip = u64::from_le_bytes([
            buffer[6], buffer[7], buffer[8], buffer[9], buffer[10], buffer[11], buffer[12],
            buffer[13],
        ]);
        assert_eq!(ip, 0x7f0000001000);

        // Verify pid/tid fields (after IP)
        let pid = u32::from_le_bytes([buffer[14], buffer[15], buffer[16], buffer[17]]);
        let tid = u32::from_le_bytes([buffer[18], buffer[19], buffer[20], buffer[21]]);
        assert_eq!(pid, 0);
        assert_eq!(tid, 0);

        // Verify time field (after TID)
        let time = u64::from_le_bytes([
            buffer[22], buffer[23], buffer[24], buffer[25], buffer[26], buffer[27], buffer[28],
            buffer[29],
        ]);
        assert_eq!(time, 0);

        // Verify period field (after time)
        let period = u64::from_le_bytes([
            buffer[30], buffer[31], buffer[32], buffer[33], buffer[34], buffer[35], buffer[36],
            buffer[37],
        ]);
        assert_eq!(period, 0);

        // Verify callchain count (after period)
        let nr = u64::from_le_bytes([
            buffer[38], buffer[39], buffer[40], buffer[41], buffer[42], buffer[43], buffer[44],
            buffer[45],
        ]);
        assert_eq!(nr, 3);

        // Verify callchain entries
        for i in 0..3 {
            let offset = 46 + (i * 8);
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
        let sample = SampleEvent::new(0, 0, 0, 0, 1000, None, None);

        let size = sample.calculate_size();
        // Header (6) + IP (8) + TID (8) + TIME (8) + PERIOD (8) = 38
        assert_eq!(size, 38);

        let mut buffer = Vec::new();
        sample.write_to(&mut buffer).unwrap();
        assert_eq!(buffer.len() as u16, size);
    }

    #[test]
    fn test_writer_with_events() {
        let mut buffer = Cursor::new(Vec::new());

        let attr = PerfEventAttr::new(0, 0, 0);
        let ids = vec![vec![1u64]];

        let mut writer = PerfDataWriter::new(&mut buffer);
        writer.initialize(&[attr], &ids).unwrap();

        // Write COMM event
        let comm = CommEvent::new(1234, 1234, "test-program".to_string());
        writer.write_comm(&comm).unwrap();

        // Write MMAP event
        let mmap = MmapEvent::new(1234, 1234, 0x400000, 0x1000, 0, "/usr/bin/test".to_string());
        writer.write_mmap(&mmap).unwrap();

        // Write SAMPLE event
        let sample = SampleEvent::new(0, 0x400123, 1234, 1234, 1000, None, None);
        writer.write_sample(&sample).unwrap();

        writer.write_finished_round().unwrap();
        writer.align_to_8_bytes().unwrap();
        writer.finalize().unwrap();

        let data = buffer.into_inner();
        assert_eq!(&data[0..8], b"PERFILE2");
        assert!(data.len() > 200); // Should have substantial content

        #[cfg(feature = "evidence")]
        {
            let _ = std::fs::write(".sisyphus/evidence/test-events.perf.data", &data);
        }
    }
}

// Temporary stub types for backward compatibility during transition
// TODO: Remove these in Task 4 when proper integration is done

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Event {
    Sample(SampleEvent),
    Mmap(MmapEvent),
    Comm(CommEvent),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PerfDataReader<R> {
    _phantom: std::marker::PhantomData<R>,
    sample_count: u64,
    mmap_count: u64,
    comm_count: u64,
}

impl<R> PerfDataReader<R> {
    #[allow(dead_code)]
    pub fn new(_reader: R) -> Result<Self, PerfError> {
        Err(PerfError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "PerfDataReader not yet implemented for Linux perf format",
        )))
    }

    #[allow(dead_code)]
    pub fn header(&self) -> PerfFileHeader {
        PerfFileHeader {
            sample_count: 0,
            mmap_count: 0,
            comm_count: 0,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn read_event(&mut self) -> Result<Option<Event>, PerfError> {
        Err(PerfError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "PerfDataReader not yet implemented for Linux perf format",
        )))
    }

    #[allow(dead_code)]
    pub fn read_all_events(&mut self) -> Result<Vec<Event>, PerfError> {
        Err(PerfError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "PerfDataReader not yet implemented for Linux perf format",
        )))
    }
}

impl PerfDataReader<File> {
    #[allow(dead_code)]
    pub fn from_path<P: AsRef<Path>>(_path: P) -> Result<Self, PerfError> {
        Err(PerfError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "PerfDataReader not yet implemented for Linux perf format",
        )))
    }
}
