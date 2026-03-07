# Learnings - linux-perf-reader

## 2026-03-07 - Task 1: Core Reader Infrastructure

### Key Findings

1. **Magic number handling**: The magic number in perf.data files is ASCII "PERFILE2" stored as 8 bytes. When reading, we need to:
    - Read as bytes for validation
    - Store as u64 (little-endian) for compatibility with existing code
    - Value: 0x32454c4946524550 (little-endian interpretation of "PERFILE2")

2. **PerfDataReader structure**: Successfully implemented:
    - `from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>` - for File type
    - `from_reader<R: Read + Seek>(reader: R) -> io::Result<Self>` - for any reader
    - `parse_header()` - reads 104-byte header, validates magic
    - `parse_attributes()` - reads attr section with event IDs
    - Accessors for header, attrs, event_ids, data_offset, data_size

3. **PerfEventAttr::read_from()**: Implemented to read 136-byte attribute structure
    - Reads all fields in little-endian order
    - Includes bitfields as u64
    - Matches kernel 6.17 format

4. **Test issues with reference files**: Tests fail with "UnexpectedEof" when reading real perf.data files
    - Reference files are 13KB-43KB, valid perf.data format
    - Issue likely in attribute section parsing or file structure interpretation
    - Need to investigate perf tool's actual file layout vs. our understanding

### Code Structure

- Added PerfDataReader struct (lines 1318-1325)
- Added from_reader() implementation (lines 1327-1343)
- Added parse_header() implementation (lines 1346-1357)
- Added parse_attributes() implementation (lines 1360-1397)
- Added accessors and utility methods (lines 1399-1436)
- Added PerfEventAttr::read_from() (lines 380-406)
- Added temporary stub Event enum for compatibility (lines 1450-1458)

### Dependencies

- byteorder crate: used for LittleEndian reading
- std::io::{Read, Seek, SeekFrom}: for file operations
- crate::error::PerfError: for error handling

### Next Steps

Task 2: Implement Event Parsers
- Need to implement SAMPLE, MMAP, COMM, FINISHED_ROUND event parsers
- Must handle variable-length events and alignment
- Must test with reference files or generated test data

## 2026-03-07 - Task 2: Event Parsers

### Implementation Summary

Successfully implemented all required event parsers for PerfDataReader:

1. **Helper function**: `read_null_terminated_padded_string<R: Read>()`
    - Reads null-terminated strings with padding to specified alignment
    - Validates UTF-8 encoding
    - Handles 8-byte alignment for perf.data format

2. **FinishedRoundEvent::read_from()**
    - Simplest event: only contains PerfEventHeader (6 bytes)
    - Used to mark boundaries in event stream where no reordering occurs

3. **MmapEvent::read_from()**
    - Reads pid, tid (4 bytes each)
    - Reads addr, len, pgoff (8 bytes each)
    - Reads filename (null-terminated, 8-byte padded)

4. **CommEvent::read_from()**
    - Reads pid, tid (4 bytes each)
    - Reads comm string (null-terminated, 8-byte padded)

5. **SampleEvent::read_from()**
    - Takes `sample_type: u64` parameter to determine which fields are present
    - Conditionally reads fields based on sample_type bitmask:
      - PERF_SAMPLE_IP: 8 bytes
      - PERF_SAMPLE_TID: 8 bytes (pid + tid)
      - PERF_SAMPLE_TIME: 8 bytes
      - PERF_SAMPLE_PERIOD: 8 bytes
      - PERF_SAMPLE_CALLCHAIN: nr (8 bytes) + nr*8 bytes for addresses
    - Properly handles variable-length callchain arrays

### Key Design Decisions

1. **SampleEvent::new() signature change**: Added `sample_type: u64` as first parameter
    - Required for reading SampleEvent since fields are variable
    - Also fixed record.rs to include sample_type when creating samples
    - Updated all test cases to include sample_type parameter

2. **String handling**: Both MmapEvent and CommEvent use 8-byte alignment for strings
    - Null-terminated strings are padded to 8-byte boundaries
    - Helper function handles both reading and writing with same alignment rules

3. **Little-endian consistency**: All parsers use byteorder::LittleEndian
    - Matches Linux kernel's byte order
    - Explicit reading ensures cross-platform compatibility

### Test Coverage

Added comprehensive unit tests for all event parsers:
- `test_finished_round_read_write()` - round-trip test
- `test_mmap_event_read_write()` - full event with filename
- `test_mmap_event_short_filename()` - minimal filename with padding
- `test_comm_event_read_write()` - full comm string
- `test_comm_event_short()` - minimal comm with padding
- `test_sample_event_read_write_basic()` - sample without callchain
- `test_sample_event_read_write_with_callchain()` - sample with callchain
- `test_sample_event_variable_fields()` - sample with only IP field
- `test_read_null_terminated_padded_string()` - helper function test

### Verification Results

- **cargo build**: Success (only dead code warnings)
- **cargo test**: All tests pass (31 total tests)
  - 22 integration tests (commands)
  - 9 perf compatibility tests (1 passed, 8 ignored due to permissions)
  - 8 perf reader unit tests
  - 1 doc test

### Updated Files

1. **src/core/perf_data.rs**:
   - Added `read_null_terminated_padded_string()` helper
   - Added `FinishedRoundEvent::read_from()`
   - Added `MmapEvent::read_from()`
   - Added `CommEvent::read_from()`
   - Added `SampleEvent::read_from(sample_type: u64)`
   - Updated `SampleEvent::new()` to accept sample_type parameter
   - Added 9 new unit tests for event parsing

2. **src/commands/record.rs**:
   - Updated `parse_sample_record()` to include sample_type parameter
   - Fixed SampleEvent::new() call with proper sample_type bitmask

3. **src/commands/report.rs**:
   - Updated test cases to include sample_type parameter
   - Fixed all SampleEvent::new() calls in tests

### Remaining Work

Task 3: Implement full event iteration in PerfDataReader
- Need to implement `read_next_event()` method that reads and parses events from data section
- Should return Result<Event> where Event is an enum wrapper for all event types
- Must handle alignment after each event
- Should stop at end of data section

Task 4: Integration with report/script commands
- Replace stub Event enum with proper implementation
- Wire up PerfDataReader to report.rs and script.rs
- Test with real perf.data files

### Code Quality Notes

- All clippy warnings are pre-existing (dead code warnings for unused constants)
- No new clippy warnings introduced by event parser implementation
- Code follows existing patterns and conventions in the codebase
- Proper error handling with io::Result throughout
- Consistent use of byteorder crate for little-endian reading