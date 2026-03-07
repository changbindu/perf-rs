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