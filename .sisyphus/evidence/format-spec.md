# perf.data File Format Specification

**Version:** Based on Linux kernel 6.17
**Last Updated:** 2026-03-07
**Reference Files:**
- `reference-empty.perf.data` - Empty recording (17 samples)
- `reference-simple.perf.data` - Simple command `ls -la` (18 samples)
- `reference-multithread.perf.data` - Multi-threaded program (786 samples)

---

## Overview

The `perf.data` file format is a binary file format used by the Linux `perf` tool to store performance monitoring data. It consists of:
1. A file header containing metadata
2. An array of event attributes
3. A data section containing event records
4. Optional feature sections after the data section

All fields are in **little-endian** byte order on the machine that generated the `perf.data` file.

---

## File Header

The file header is the first 104 bytes of the file.

### Header Structure (104 bytes total)

```c
struct perf_file_header {
    u64    magic;                      // Offset 0, Size 8
    u64    size;                       // Offset 8, Size 8
    u64    attr_size;                  // Offset 16, Size 8
    struct perf_file_section attrs;    // Offset 24, Size 16
    struct perf_file_section data;     // Offset 40, Size 16
    struct perf_file_section event_types; // Offset 56, Size 16
    u64    flags;                      // Offset 72, Size 8
    u64    flags1[3];                  // Offset 80, Size 24
};
```

### Field Details

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x00 | 8 | `magic` | Magic number: **0x50455246494c4532** ("PERFILE2" in ASCII) |
| 0x08 | 8 | `size` | Size of the header (104 bytes) |
| 0x10 | 8 | `attr_size` | Size of each attribute entry in attrs section |
| 0x18 | 16 | `attrs` | Section containing perf_event_attr structures |
| 0x28 | 16 | `data` | Section containing event data records |
| 0x38 | 16 | `event_types` | Ignored (reserved) |
| 0x48 | 8 | `flags` | Bitmap of enabled features (256 bits) |
| 0x50 | 24 | `flags1` | Additional flags (extension of flags) |

### perf_file_section Structure

```c
struct perf_file_section {
    u64 offset;    // Offset from start of file
    u64 size;      // Size of the section in bytes
};
```

---

## Binary Analysis of Reference Files

### reference-empty.perf.data

```
00000000: 5045 5246 494c 4532 6800 0000 0000 0000  PERFILE2h.......
00000010: 9800 0000 0000 0000 a800 0000 0000 0000  ................
00000020: 9800 0000 0000 0000 4001 0000 0000 0000  ........@.......
00000030: d808 0000 0000 0000 0000 0000 0000 0000  ................
00000040: 0000 0000 0000 0000 fc7f 7196 0000 0000  ..........q.....
00000050: 0000 0000 0000 0000 0000 0000 0000 0000  ................
00000060: 0000 0000 0000 0000 3b03 0000 0000 0000  ........;.......
```

**Field values:**
- `magic`: 0x50455246494c4532 ("PERFILE2")
- `size`: 0x68 = 104 bytes
- `attr_size`: 0x98 = 152 bytes (includes perf_event_attr + file section)
- `attrs.offset`: 0xa8 = 168 bytes
- `attrs.size`: 0x98 = 152 bytes
- `data.offset`: 0x980 = 2432 bytes
- `data.size`: 0x8d8 = 2256 bytes

---

## Attributes Section

The attributes section contains an array of `perf_event_attr` structures, each followed by a `perf_file_section` pointing to the array of event IDs.

### perf_event_attr Structure (136 bytes in kernel 6.17)

```c
struct perf_event_attr {
    __u32   type;                  // Major type: hardware/software/tracepoint/etc
    __u32   size;                  // Size of this struct
    __u64   config;                // Type-specific configuration
    union {
        __u64   sample_period;     // Sampling period
        __u64   sample_freq;       // Sampling frequency
    };
    __u64   sample_type;           // What to include in samples
    __u64   read_format;           // Format of read() data
    __u64   disabled :  1,         // Event attributes (bit fields)
            inherit :  1,
            pinned :  1,
            exclusive :  1,
            exclude_user :  1,
            exclude_kernel :  1,
            exclude_hv :  1,
            exclude_idle :  1,
            mmap :  1,
            comm :  1,
            freq :  1,
            inherit_stat :  1,
            enable_on_exec :  1,
            task :  1,
            watermark :  1,
            precise_ip :  2,
            mmap_data :  1,
            sample_id_all :  1,
            exclude_host :  1,
            exclude_guest :  1,
            exclude_callchain_kernel :  1,
            exclude_callchain_user :  1,
            mmap2 :  1,
            comm_exec :  1,
            use_clockid :  1,
            context_switch :  1,
            write_backward :  1,
            namespaces :  1,
            ksymbol :  1,
            bpf_event :  1,
            aux_output :  1,
            cgroup :  1,
            text_poke :  1,
            build_id :  1,
            inherit_thread :  1,
            remove_on_exec :  1,
            sigtrap :  1,
            defer_callchain :  1,
            defer_output :  1,
            __reserved_1 : 24;
    union {
        __u32   wakeup_events;     // Wake up every n events
        __u32   wakeup_watermark;  // Bytes before wakeup
    };
    __u32   bp_type;
    union {
        __u64   bp_addr;
        __u64   kprobe_func;
        __u64   uprobe_path;
        __u64   config1;
    };
    union {
        __u64   bp_len;
        __u64   kprobe_addr;
        __u64   probe_offset;
        __u64   config2;
    };
    __u64   branch_sample_type;
    __u64   sample_regs_user;
    __u32   sample_stack_user;
    __s32   clockid;
    __u64   sample_regs_intr;
    __u32   aux_watermark;
    __u16   sample_max_stack;
    __u16   __reserved_2;
    __u32   aux_sample_size;
    union {
        __u32   aux_action;
        struct {
            __u32   aux_start_paused :  1,
                    aux_pause :  1,
                    aux_resume :  1,
                    __reserved_3 : 29;
        };
    };
    __u64   sig_data;
    __u64   config3;
    __u64   config4;
};
```

### Attribute Entry Format

Each entry in the attrs section:

```c
struct {
    struct perf_event_attr attr;    // Size: header.attr_size (136 bytes)
    struct perf_file_section ids;   // Offset and size of ID array
};
```

The `ids` section points to an array of `u64` values, each representing a unique event ID.

---

## Data Section

The data section contains a stream of event records matching the format generated by the kernel.

### perf_event_header Structure

All event records start with a common header:

```c
struct perf_event_header {
    __u16 type;   // Event type (see below)
    __u16 misc;   // Additional flags
    __u16 size;   // Total size of the event record including header
};
```

### Alignment

**Important:** All event records are aligned to **8-byte boundaries**.

### Event Types

#### Kernel Event Types (0-63)

| Type | Name | Description |
|------|------|-------------|
| 0 | | Reserved |
| 1 | PERF_RECORD_MMAP | Memory mapping event |
| 2 | PERF_RECORD_LOST | Lost samples event |
| 3 | PERF_RECORD_COMM | Command name (exec) event |
| 4 | PERF_RECORD_EXIT | Process exit event |
| 5 | PERF_RECORD_THROTTLE | Event throttled |
| 6 | PERF_RECORD_UNTHROTTLE | Event unthrottled |
| 7 | PERF_RECORD_FORK | Process fork event |
| 8 | PERF_RECORD_READ | Counter read event |
| 9 | PERF_RECORD_SAMPLE | Sample event |
| 10 | PERF_RECORD_MMAP2 | Extended MMAP with inode data |
| 11 | PERF_RECORD_AUX | AUX area data |
| 12 | PERF_RECORD_ITRACE_START | Instruction trace started |
| 13 | PERF_RECORD_LOST_SAMPLES | Dropped samples event |
| 14 | PERF_RECORD_SWITCH | Context switch event |
| 15 | PERF_RECORD_SWITCH_CPU_WIDE | CPU-wide context switch |
| 16 | PERF_RECORD_NAMESPACES | Namespace event |
| 17 | PERF_RECORD_KSYMBOL | Kernel symbol event |
| 18 | PERF_RECORD_BPF_EVENT | BPF event |
| 19 | PERF_RECORD_CGROUP | Cgroup event |
| 20 | PERF_RECORD_TEXT_POKE | Text modification event |
| 21 | PERF_RECORD_AUX_OUTPUT_HW_ID | HW ID for AUX output |
| 22 | PERF_RECORD_CALLCHAIN_DEFERRED | Deferred callchain |

#### User Event Types (64+)

| Type | Name | Description |
|------|------|-------------|
| 64 | PERF_RECORD_HEADER_ATTR | Attribute definition |
| 65 | PERF_RECORD_HEADER_EVENT_TYPE | Event type (deprecated) |
| 66 | PERF_RECORD_HEADER_TRACING_DATA | Tracing data |
| 67 | PERF_RECORD_HEADER_BUILD_ID | Build ID event |
| 68 | PERF_RECORD_FINISHED_ROUND | End of round (no reordering) |
| 69 | PERF_RECORD_ID_INDEX | ID to CPU/TID mapping |
| 70 | PERF_RECORD_AUXTRACE_INFO | Auxtrace info |
| 71 | PERF_RECORD_AUXTRACE | Auxtrace data |
| 72 | PERF_RECORD_AUXTRACE_ERROR | Auxtrace error |
| 80 | PERF_RECORD_HEADER_FEATURE | Header feature (pipe mode) |
| 81 | PERF_RECORD_COMPRESSED | Compressed data (deprecated) |
| 82 | PERF_RECORD_FINISHED_INIT | End of init records |
| 83 | PERF_RECORD_COMPRESSED2 | Compressed data (8-byte aligned) |

### Common Event Formats

#### PERF_RECORD_MMAP (Type 1)

```c
struct {
    struct perf_event_header header;
    u32 pid, tid;
    u64 addr;
    u64 len;
    u64 pgoff;
    char filename[];
    struct sample_id sample_id;  // If sample_id_all is set
};
```

#### PERF_RECORD_COMM (Type 3)

```c
struct {
    struct perf_event_header header;
    u32 pid, tid;
    char comm[];
    struct sample_id sample_id;  // If sample_id_all is set
};
```

#### PERF_RECORD_SAMPLE (Type 9)

This is the most common event type. The structure varies based on `sample_type` bits.

```c
struct {
    struct perf_event_header header;
    // Variable fields based on sample_type:
    { u64 ip; }                     && PERF_SAMPLE_IP
    { u32 pid, tid; }               && PERF_SAMPLE_TID
    { u64 time; }                   && PERF_SAMPLE_TIME
    { u64 addr; }                   && PERF_SAMPLE_ADDR
    { u64 id; }                     && PERF_SAMPLE_ID
    { u64 stream_id; }              && PERF_SAMPLE_STREAM_ID
    { u32 cpu, res; }               && PERF_SAMPLE_CPU
    { u64 period; }                 && PERF_SAMPLE_PERIOD
    { struct read_format values; }  && PERF_SAMPLE_READ
    { u64 nr, ips[nr]; }            && PERF_SAMPLE_CALLCHAIN
    { u32 size, data[size]; }       && PERF_SAMPLE_RAW
    { u64 nr, branches[nr]; }       && PERF_SAMPLE_BRANCH_STACK
    // ... and more fields
};
```

#### PERF_RECORD_MMAP2 (Type 10)

Extended version of MMAP with inode and build ID information:

```c
struct {
    struct perf_event_header header;
    u32 pid, tid;
    u64 addr;
    u64 len;
    u64 pgoff;
    union {
        struct {
            u32 maj;
            u32 min;
            u64 ino;
            u64 ino_generation;
        };
        struct {
            u8  build_id_size;
            u8  __reserved_1;
            u16 __reserved_2;
            u8  build_id[20];
        };
    };
    u32 prot, flags;
    char filename[];
    struct sample_id sample_id;  // If sample_id_all is set
};
```

#### PERF_RECORD_HEADER_ATTR (Type 64)

```c
struct {
    struct perf_event_header header;
    struct perf_event_attr attr;
    u64 id[];
};
```

#### PERF_RECORD_FINISHED_ROUND (Type 68)

No payload - just the header. Marks a point in the event stream where no reordering occurs across this boundary.

---

## Feature Sections

After the data section, optional feature sections may be present based on the feature bitmap in the header.

### Feature Bits (256-bit bitmap)

| Bit | Name | Description |
|-----|------|-------------|
| 0 | HEADER_RESERVED | Always cleared |
| 1 | HEADER_TRACING_DATA | Tracing data |
| 2 | HEADER_BUILD_ID | Build IDs for referenced executables |
| 3 | HEADER_HOSTNAME | Hostname where data was collected |
| 4 | HEADER_OSRELEASE | Kernel version |
| 5 | HEADER_VERSION | Perf tool version |
| 6 | HEADER_ARCH | CPU architecture |
| 7 | HEADER_NRCPUS | Number of CPUs |
| 8 | HEADER_CPUDESC | CPU description |
| 9 | HEADER_CPUID | CPU type |
| 10 | HEADER_TOTAL_MEM | Total memory |
| 11 | HEADER_CMDLINE | Command line used |
| 12 | HEADER_EVENT_DESC | Detailed event descriptions |
| 13 | HEADER_CPU_TOPOLOGY | CPU topology |
| 14 | HEADER_NUMA_TOPOLOGY | NUMA topology |
| 15 | HEADER_BRANCH_STACK | Branch stack info |
| 16 | HEADER_PMU_MAPPINGS | PMU mappings |
| 17 | HEADER_GROUP_DESC | Counter groups |
| 18 | HEADER_AUXTRACE | Auxtrace areas |
| 19 | HEADER_STAT | Stat record flag |
| 20 | HEADER_CACHE | Cache hierarchy |
| 21 | HEADER_SAMPLE_TIME | Sample time range |
| 22 | HEADER_MEM_TOPOLOGY | Memory topology |
| 23 | HEADER_CLOCKID | Clock ID frequency |
| 24 | HEADER_DIR_FORMAT | Directory format |
| 25 | HEADER_BPF_PROG_INFO | BPF program info |
| 26 | HEADER_BPF_BTF | BPF type format |
| 27 | HEADER_COMPRESSED | Compression info |
| 28 | HEADER_CPU_PMU_CAPS | CPU PMU capabilities |
| 29 | HEADER_CLOCK_DATA | Clock reference time |
| 30 | HEADER_HYBRID_TOPOLOGY | Hybrid CPU topology |
| 31 | HEADER_PMU_CAPS | PMU capabilities |
| 32 | HEADER_CPU_DOMAIN_INFO | CPU domain info |

### Common Feature Formats

#### HEADER_BUILD_ID (Bit 2)

```c
struct build_id_event {
    struct perf_event_header header;
    pid_t pid;
    uint8_t build_id[24];
    char filename[header.size - offsetof(struct build_id_event, filename)];
};
```

#### HEADER_HOSTNAME, HEADER_OSRELEASE, HEADER_VERSION, HEADER_ARCH, HEADER_CPUDESC, HEADER_CPUID

```c
struct perf_header_string {
    uint32_t len;
    char string[len];  // Null-terminated
};
```

#### HEADER_NRCPUS (Bit 7)

```c
struct nr_cpus {
    uint32_t nr_cpus_available;
    uint32_t nr_cpus_online;
};
```

#### HEADER_CMDLINE (Bit 11)

```c
struct perf_header_string_list {
    uint32_t nr;
    struct perf_header_string strings[nr];
};
```

---

## String Encoding

All strings in perf.data follow this format:

```c
struct perf_header_string {
    uint32_t len;      // Length including null terminator
    char string[len];  // Null-terminated string
};
```

Strings are **padded to 64-byte alignment** (`NAME_ALIGN`).

---

## Endian Detection

The magic number can be used to detect endianness:
- **Native endian:** 0x50455246494c4532 ("PERFILE2")
- **Swapped endian:** 0x32454c4946524550

If the magic value is byte-swapped, the file is in non-native endian and all multi-byte values need to be swapped.

---

## Record Ordering

Events in the data section are **not necessarily in timestamp order** because they can be collected in parallel on different CPUs. To process events in time order:
1. Sort events by their timestamp field
2. Use `PERF_RECORD_FINISHED_ROUND` markers for partial sorting (no reordering across these boundaries)

---

## Sample ID

When `sample_id_all` is set in the event attributes, all events include a `sample_id` structure at the end:

```c
struct sample_id {
    { u32 pid, tid; }     && PERF_SAMPLE_TID
    { u64 time; }         && PERF_SAMPLE_TIME
    { u64 id; }           && PERF_SAMPLE_ID
    { u64 stream_id; }    && PERF_SAMPLE_STREAM_ID
    { u32 cpu, res; }     && PERF_SAMPLE_CPU
    { u64 id; }           && PERF_SAMPLE_IDENTIFIER
};
```

The `PERF_SAMPLE_IDENTIFIER` field is at a fixed offset from the header, making it useful for reliable parsing when multiple event types are present.

---

## Pipe Mode

When `perf` writes to a pipe, it uses a simplified header format:

```c
struct perf_pipe_file_header {
    u64 magic;   // Same magic number
    u64 size;    // Size of this header (16 bytes)
};
```

In pipe mode, attribute and metadata information is sent as synthesized events (PERF_RECORD_HEADER_ATTR, PERF_RECORD_HEADER_FEATURE, etc.) instead of being in the file header.

---

## References

- Linux kernel source: `tools/perf/util/header.h`
- Linux kernel source: `include/uapi/linux/perf_event.h`
- Linux perf man page: `man perf_events`
- pmu-tools parser: https://github.com/andikleen/pmu-tools/tree/master/parser
- quipper C++ parser: https://github.com/google/perf_data_converter

---

## Implementation Notes for perf-rs

### Required Crates
- `byteorder` - For explicit little-endian serialization
- `memmap2` - For memory-mapped file I/O
- `thiserror` - For error handling

### Key Requirements
1. **8-byte alignment** for all records
2. **Little-endian byte order** (explicit, not relying on native endianness)
3. **Magic number check** and endian detection
4. **String null-termination** and 64-byte padding
5. **Feature bitmap parsing** (256 bits)
6. **Sample ID handling** with variable fields

### Header Implementation

```rust
pub const PERF_FILE_MAGIC: u64 = 0x50455246494c4532; // "PERFILE2"
pub const PERF_FILE_HEADER_SIZE: u64 = 104;

#[repr(C)]
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

#[repr(C)]
pub struct PerfFileSection {
    pub offset: u64,
    pub size: u64,
}
```

### perf_event_attr Implementation

Note: The size field in `perf_event_attr` indicates the actual size of the structure (varies by kernel version). Kernel 6.17 uses size 136 bytes.

```rust
pub const PERF_ATTR_SIZE_VER8: u32 = 136; // Kernel 6.17

#[repr(C)]
pub struct PerfEventAttr {
    pub type_: u32,
    pub size: u32,
    pub config: u64,
    pub sample_period_or_freq: u64,
    pub sample_type: u64,
    pub read_format: u64,
    pub bitfields: u64,
    pub wakeup_events_or_watermark: u32,
    pub bp_type: u32,
    pub config1_or_bp_addr_or_func: u64,
    pub config2_or_bp_len: u64,
    pub branch_sample_type: u64,
    pub sample_regs_user: u64,
    pub sample_stack_user: u32,
    pub clockid: i32,
    pub sample_regs_intr: u64,
    pub aux_watermark: u32,
    pub sample_max_stack: u16,
    pub _reserved_2: u16,
    pub aux_sample_size: u32,
    pub aux_action: u32,
    pub sig_data: u64,
    pub config3: u64,
    pub config4: u64,
}
```

---

## Verification Commands

The reference files can be verified with:

```bash
# Read header
perf report -i reference-simple.perf.data --header-only

# Display events
perf script -i reference-simple.perf.data

# Hex dump
xxd reference-simple.perf.data | head -30
```

---

## File Size Summary

| File | Description | Size |
|------|-------------|------|
| reference-empty.perf.data | Empty recording (17 samples) | 2.5 KB |
| reference-simple.perf.data | Simple command `ls -la` (18 samples) | 2.5 KB |
| reference-multithread.perf.data | Multi-threaded program (786 samples) | 32 KB |

---

## Version History

| Version | Kernel | Key Changes |
|---------|--------|-------------|
| 1 | < 2.6.32 | PERFFILE magic, old format |
| 2 | 2.6.32+ | PERFILE2 magic, current format |

Current perf-rs should target **Version 2** (PERFILE2 magic).