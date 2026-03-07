# Learnings - perf-data-compatibility

## 2026-03-07 - Task 1: Research perf.data Format

### Key Format Details Discovered

1. **Magic Number**: PERFILE2 (0x50455246494c4532) - NOT "PERFRS01" used in current code
2. **Header Size**: 104 bytes fixed (NOT 64 bytes as currently implemented)
3. **perf_event_attr**: 136 bytes in kernel 6.17, variable size across kernel versions
4. **Event Types**: Linux perf uses different type numbers:
   - PERF_RECORD_MMAP = 1 (matches current)
   - PERF_RECORD_COMM = 3 (current uses 2)
   - PERF_RECORD_SAMPLE = 9 (current uses 3)
5. **Alignment**: ALL records must be 8-byte aligned
6. **Byte Order**: Little-endian, explicit serialization required
7. **String Encoding**: Length-prefixed, null-terminated, padded to 64 bytes

### Current Code Status

- `src/core/perf_data.rs` uses CUSTOM format - NOT Linux perf compatible
- Must COMPLETELY rewrite this file
- Event type numbers are wrong
- Header structure is wrong
- Missing: perf_event_attr array, feature bits section, proper data section

### Reference Files Created

- `reference-empty.perf.data`: 17 samples, 2.5 KB
- `reference-simple.perf.data`: 18 samples, 2.5 KB (ls -la)
- `reference-multithread.perf.data`: 786 samples, 32 KB

All verified with `perf report` and `perf script`.

### Dependencies

- `byteorder` crate required for explicit little-endian serialization
- `linux-perf-data = "0.10"` already in Cargo.toml but NOT used
- Could potentially use `linux-perf-data` crate for reading Linux format

### Environment

- perf version: 6.17.9
- kernel: 6.17.0-14-generic
- CPU: Intel Core i5-8265U (x86_64)

### Implementation Approach

- Implement from scratch using format specification
- Use `byteorder` crate for explicit little-endian
- 8-byte alignment for all records
- Follow Linux kernel structure definitions exactly

## 2026-03-07 - Task 5: Validation Tests

### Test Suite Implementation

1. **Test File Created**: `tests/perf_compatibility.rs` with comprehensive validation tests
2. **Test Categories**:
   - Empty recording (very short duration with `true`)
   - Simple command (`ls -la`)
   - Multi-threaded application (Rust program spawning 4 threads)
   - Large file (high frequency recording at 1000 Hz)
   - Very short duration (edge case with `sh -c ":"`)
   - Specific event (instructions)
   - Sample period (instead of frequency)
   - System-wide recording (all CPUs)

3. **Validation Approach**:
   - Verify perf.data magic number (PERFILE2)
   - Run `perf report` on generated files
   - Run `perf script` on generated files
   - Check for errors in stderr output
   - Validate file sizes and sample counts

4. **Permission Handling**:
   - Tests use `#[ignore]` attribute requiring root/CAP_SYS_ADMIN
   - Helper function `has_perf_permission()` checks privileges
   - Helper function `perf_available()` checks if perf tool is installed

5. **Multi-threaded Test**:
   - Dynamically compiles a Rust test program
   - Spawns 4 threads performing computations
   - Validates samples from multiple threads

### Test Helper Functions

- `run_perf_rs()`: Execute perf-rs with arguments
- `verify_perf_data_magic()`: Check PERFILE2 magic number
- `perf_report()`: Run perf report and check for errors
- `perf_script()`: Run perf script and check for errors
- `has_perf_permission()`: Check perf event permissions
- `perf_available()`: Check if perf tool is available

### Testing Challenges

1. **System-wide Recording**: Complex to automate - requires background process and SIGINT
2. **Multi-threaded Test**: Requires rustc and temporary file compilation
3. **High Frequency Tests**: May produce very large files
4. **Permission Requirements**: All tests require elevated privileges

### Build Status

- Test file compiles successfully
- All tests marked with `#[ignore]` to run manually with permissions
- Uses `tempfile` crate for temporary directories
- No new dependencies required (tempfile already in dev-dependencies)