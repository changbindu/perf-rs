# Linux perf Compatible perf.data Generation

## TL;DR

> **Goal**: Implement Linux perf compatible perf.data file writer to enable interoperability with standard perf tools (`perf report`, `perf script`, etc.).

> **Deliverables**:
> - Linux perf format writer in `src/core/perf_data.rs`
> - Support for event types: SAMPLE, MMAP, COMM, FINISHED_ROUND
> - Integration with `record` command
> - Validation tests with real `perf` tools

> **Estimated Effort**: Large

---

## Context

### Original Request
"The most important feature is perf-rs can generate linux perf compatible perf.data file."

### Key Requirements
- **Compatibility**: Full compatibility with Linux perf tools (perf report, perf script)
- **Event Support**: Core events first: SAMPLE (IP|TID|TIME|PERIOD), MMAP, COMM, FINISHED_ROUND
- **Implementation**: Complete rewrite of `perf_data.rs` module
- **Validation**: Test with real `perf report` and `perf script` commands

### Implementation Constraints
- **Must Use**: `byteorder` crate for explicit little-endian serialization
- **Must Implement**: 8-byte alignment for all records
- **Must NOT**: Assume struct layout matches binary format
- **Magic Number**: PERFILE2 (0x32454c4946524550ULL)
- **Header Size**: 104 bytes
- Do not create new worktrees.
---

## Work Objectives

### Core Objective
Implement Linux perf compatible perf.data file writer that generates files readable by standard Linux perf tools.

### Definition of Done
- [ ] `perf report -i output.perf.data --stdio` exits successfully
- [ ] `perf script -i output.perf.data` shows sample records
- [ ] Integration test: `cargo run -- record -o test.perf.data -- ls` produces valid file
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)

---

## TODOs

- [x] 1. Research perf.data Format
   - Study Linux kernel source: `tools/perf/util/header.h`, `include/uapi/linux/perf_event.h`
   - Create reference perf.data files using real `perf record` command
   - Document binary layout: header (104 bytes), perf_event_attr (136 bytes), event records
   - Verify environment: perf tool availability, kernel version, capabilities

   **Acceptance Criteria**:
   - [x] Format spec document in `.sisyphus/evidence/format-spec.md`
   - [x] 3 reference files created (empty, simple command, multi-threaded)
   - [x] Environment validated

- [x] 2. Implement perf.data Writer Core
   - Implement file header (PERFILE2 magic, 104 bytes)
   - Implement attributes section (perf_event_attr structure)
   - Implement FINISHED_ROUND event
   - Use `byteorder` crate for explicit little-endian serialization
   - Ensure 8-byte alignment for all records

   **Acceptance Criteria**:
   - [x] `src/core/perf_data.rs` rewritten with Linux perf structures
   - [x] Test: `xxd -l 8 output.perf.data` shows `PERFILE2` magic
   - [x] Test: minimal valid file accepted by `perf report`

   **Commit**: `feat(core): implement Linux perf file header and attributes`

- [x] 3. Implement Event Writers
   - Implement SAMPLE event (IP, TID, TIME, PERIOD fields)
   - Implement MMAP event (pid, tid, addr, len, pgoff, filename)
   - Implement COMM event (pid, tid, comm string)
   - Handle bit-ordered sample fields
   - Ensure proper string null-termination and padding

   **Acceptance Criteria**:
   - [x] SAMPLE event writer implemented
   - [x] MMAP event writer implemented
   - [x] COMM event writer implemented
   - [x] Test: `perf script -i output.perf.data` shows samples and process names

   **Commit**: `feat(core): implement SAMPLE, MMAP, COMM event writers`

- [ ] 4. Integrate with Record Command
  - Modify `src/commands/record.rs` to use new writer
  - Replace old PerfDataWriter with Linux perf writer
  - Ensure MMAP/COMM events are captured or synthesized
  - Wire up event flow from ring buffer to file

  **Acceptance Criteria**:
  - [ ] Record command uses new writer
  - [ ] Test: `cargo run -- record -o test.perf.data -- ls` produces valid file
  - [ ] Test: `perf report -i test.perf.data --stdio` shows results

  **Commit**: `feat(commands): integrate Linux perf writer with record command`

- [x] 5. Create Validation Tests
   - Test: empty recording
   - Test: simple command (ls)
   - Test: multi-threaded application
   - Validate with `perf report` and `perf script`
   - Test large files and edge cases

   **Acceptance Criteria**:
   - [x] Test suite in `tests/perf_compatibility.rs`
   - [x] All tests pass: `cargo test --test perf_compatibility`
   - [x] Real perf tools can read all test files

   **Commit**: `test: add perf.data compatibility validation tests`

---

## References

**Primary Sources**:
- Linux kernel: `tools/perf/util/header.h` - Header structure definitions
- Linux kernel: `include/uapi/linux/perf_event.h` - Event type definitions
- `linux-perf-data` crate: https://crates.io/crates/linux-perf-data (read-only parser reference)

**Key Format Details**:
- File header: 104 bytes
- Magic number: PERFILE2 (0x32454c4946524550ULL)
- perf_event_attr: 136 bytes
- Sample fields ordered by bit number
- All records 8-byte aligned
- Little-endian byte order
- Strings null-terminated and padded to 8 bytes
