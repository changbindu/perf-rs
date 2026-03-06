# System-Level Profiling for record and stat Commands

## TL;DR

> **Quick Summary**: Add system-wide profiling capability to `perf stat` and `perf record` commands, allowing monitoring of all CPUs and all processes (similar to `perf stat -a` and `perf record -a`).
> 
> **Deliverables**:
> - `-a/--all-cpus` flag for system-wide profiling
> - `-C/--cpu` option for selecting specific CPUs
> - `--per-cpu` flag for per-CPU breakdown in stat output
> - System-wide privilege checking
> - CPU detection and selection utilities
> - Per-CPU ring buffer management for record
> - TDD test coverage
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: CPU utilities → PerfConfig extension → stat implementation → record implementation → integration tests

---

## Context

### Original Request
Add system-level profiling to the record and stat subcommands.

### Interview Summary
**Key Discussions**:
- User wants true system-wide profiling (all CPUs, all processes)
- CLI interface: `-a/--all-cpus` flag (like `perf stat -a`)
- Also add `-C/--cpu` option for selecting specific CPUs (comma-separated or ranges)
- Stat output: aggregated by default, `--per-cpu` flag for per-CPU breakdown
- Record output: single perf.data file from all CPUs
- Test strategy: TDD (Test-Driven Development)

**Research Findings**:
- Current implementation only supports process-level profiling (pid-based)
- `PerfConfig` has `cpu: Option<u32>` field but no `with_cpu()` builder method
- `RingBuffer` only has `from_event_for_pid()`, no CPU-wide method
- CLI lacks `-a/--all-cpus` and `-C/--cpu` options
- `perf_event_open()` syscall: `pid=-1, cpu=N` monitors all processes on CPU N
- System-wide requires `perf_event_paranoid ≤ 0` or CAP_PERFMON/CAP_SYS_ADMIN
- Need to create one perf event per CPU and aggregate results

### Metis Review
**Identified Gaps** (addressed):
- CLI flag interaction (-a and -C): Made mutually exclusive with clear error message
- Ring buffer architecture: One buffer per CPU with sequential polling (simpler approach)
- Per-CPU output format: Table format like `perf stat --per-cpu`
- CPU selection syntax: Support list, range, and mixed formats
- Privilege checking: Add `can_profile_system_wide()` method
- Perf.data compatibility: Add PERF_SAMPLE_CPU to sample metadata for standard tools
- Resource exhaustion: Warn on large CPU counts, document buffer size implications

**Guardrails Applied**:
- MUST NOT add process/thread filtering (separate feature)
- MUST NOT add event auto-selection or smart defaults
- MUST NOT introduce async/threading without explicit request
- MUST NOT modify existing CLI flag behaviors
- MUST maintain perf.data format compatibility with standard tools

---

## Work Objectives

### Core Objective
Enable system-wide performance monitoring in `perf stat` and `perf record` commands, allowing users to profile all activity on selected CPUs without attaching to a specific process.

### Concrete Deliverables
- CPU detection utility module (`src/core/cpu.rs`)
- Extended `PerfConfig` with `with_cpu()` and `with_all_cpus()` builders
- Extended `RingBuffer` with CPU-wide creation method
- CLI flags: `-a/--all-cpus`, `-C/--cpu <list>`, `--per-cpu` (stat only)
- System-wide mode in `stat` command with aggregated and per-CPU output
- System-wide mode in `record` command with single perf.data output
- Privilege check extension for system-wide requirements
- Unit tests and integration tests (TDD approach)

### Definition of Done
- [ ] `cargo test` passes all unit tests
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt -- --check` passes
- [ ] Integration tests pass with root privileges (`cargo test -- --ignored`)
- [ ] System-wide stat produces correct aggregated output
- [ ] System-wide stat with `--per-cpu` shows per-CPU breakdown
- [ ] System-wide record creates valid perf.data file
- [ ] perf.data from system-wide recording is readable by `perf report`
- [ ] Error messages are clear for privilege and CPU validation failures

### Must Have
- CPU detection and selection utilities with tests
- `with_cpu()` builder method on PerfConfig
- CPU-wide ring buffer creation for record
- `-a/--all-cpus` flag for stat and record
- `-C/--cpu` option supporting list, range, and mixed syntax
- `--per-cpu` flag for stat command
- Privilege check for system-wide profiling
- Clear error messages for insufficient privileges
- Unit tests for all new utilities
- Integration tests for system-wide modes

### Must NOT Have (Guardrails from Metis)
- Process/thread filtering (separate feature)
- Event auto-selection or smart defaults
- Real-time streaming or visualization
- BPF integration
- Hardware counter multiplexing (handle via kernel)
- Docker/container awareness
- NUMA-aware CPU selection
- Async/threading complexity (use sequential polling)
- Modified perf.data format (must be compatible)
- Changes to existing CLI flag behaviors

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test)
- **Automated tests**: TDD
- **Framework**: cargo test (built-in Rust test framework)
- **TDD**: Each task follows RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Library/Module**: Use Bash (cargo test) — Run tests, verify pass/fail
- **CLI Integration**: Use Bash — Run commands with sudo, check output and exit codes
- **Error Cases**: Use Bash — Run commands without privileges, verify error messages

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — foundation utilities):
├── Task 1: CPU detection utility with tests [quick]
├── Task 2: CPU list parser with tests [quick]
├── Task 3: Privilege check extension [quick]
└── Task 4: PerfConfig with_cpu() builder with tests [quick]

Wave 2 (After Wave 1 — core infrastructure):
├── Task 5: RingBuffer CPU-wide creation method with tests [quick]
├── Task 6: CLI flags for stat (-a, -C, --per-cpu) [quick]
└── Task 7: CLI flags for record (-a, -C) [quick]

Wave 3 (After Wave 2 — command implementation):
├── Task 8: System-wide stat implementation (aggregated) [deep]
├── Task 9: System-wide stat --per-cpu output [deep]
└── Task 10: System-wide record implementation [deep]

Wave 4 (After Wave 3 — verification):
├── Task 11: Integration tests for stat [deep]
├── Task 12: Integration tests for record [deep]
├── Task 13: Perf.data compatibility verification [deep]
└── Task 14: Error handling and edge cases [unspecified-high]

Wave FINAL (After ALL tasks — independent review):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)

Critical Path: T1 → T4 → T8 → T11 → F1-F4
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 4 (Wave 1)
```

### Dependency Matrix

- **1-4**: — — 8-10, 1
- **5**: 4 — 10, 2
- **6**: 1, 2 — 8-9, 2
- **7**: 1, 2 — 10, 2
- **8**: 3, 4, 6 — 11, 2
- **9**: 8 — 11, 2
- **10**: 3, 4, 5, 7 — 12-13, 2
- **11**: 8, 9 — F1-F4, 1
- **12**: 10 — F1-F4, 1
- **13**: 10 — F1-F4, 1
- **14**: 8-10 — F1-F4, 1

### Agent Dispatch Summary

- **1**: **4** — T1-T4 → `quick`
- **2**: **3** — T5 → `quick`, T6-T7 → `quick`
- **3**: **3** — T8-T9 → `deep`, T10 → `deep`
- **4**: **4** — T11-T12 → `deep`, T13 → `deep`, T14 → `unspecified-high`
- **FINAL**: **4** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task MUST have: Recommended Agent Profile + Parallelization info + QA Scenarios.

- [x] 1. CPU Detection Utility

  **What to do**:
  - Create new module `src/core/cpu.rs`
  - Implement `get_cpu_count() -> Result<usize>` to detect number of online CPUs
  - Implement `get_online_cpus() -> Result<Vec<u32>>` to get list of online CPU IDs
  - Add unit tests for CPU detection
  - Export from `src/core/mod.rs`

  **Must NOT do**:
  - Don't add CPU topology or NUMA awareness (out of scope)
  - Don't cache CPU count (re-read on each call for hot-plug scenarios)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Straightforward utility module with clear requirements
  - **Skills**: []
    - No special skills needed for this task

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Tasks 6, 7, 8, 10
  - **Blocked By**: None

  **References**:
  - `src/core/mod.rs` - Module structure and export pattern
  - `src/core/privilege.rs:120-144` - Pattern for system info queries
  - `/proc/cpuinfo` or `/sys/devices/system/cpu/online` - Linux CPU info sources
  - `nix` crate - System call wrappers

  **Why Each Reference Matters**:
  - `src/core/mod.rs`: Shows how to add new module and export public API
  - `privilege.rs`: Demonstrates pattern for system-level queries
  - `/proc` and `/sys`: Linux kernel interfaces for CPU information
  - `nix` crate: May have CPU affinity functions to reference

  **Acceptance Criteria**:
  - [ ] `src/core/cpu.rs` created with `get_cpu_count()` and `get_online_cpus()`
  - [ ] Unit tests: `test_get_cpu_count()`, `test_get_online_cpus()`
  - [ ] `cargo test --lib cpu` passes
  - [ ] Module exported from `src/core/mod.rs`

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: CPU detection returns valid count
    Tool: Bash
    Preconditions: System has at least 1 CPU
    Steps:
      1. cargo test test_get_cpu_count -- --nocapture
      2. Assert output contains "test result: ok"
    Expected Result: Test passes, CPU count is > 0
    Failure Indicators: Test fails or panics
    Evidence: .sisyphus/evidence/task-01-cpu-detection.txt

  Scenario: Online CPUs list is valid
    Tool: Bash
    Preconditions: System has online CPUs
    Steps:
      1. cargo test test_get_online_cpus -- --nocapture
      2. Assert output contains "test result: ok"
    Expected Result: Test passes, online CPUs list is non-empty
    Failure Indicators: Test fails or returns empty list
    Evidence: .sisyphus/evidence/task-01-online-cpus.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add CPU detection utility`
  - Files: `src/core/cpu.rs`, `src/core/mod.rs`
  - Pre-commit: `cargo test --lib cpu`

- [x] 2. CPU List Parser

  **What to do**:
  - Add CPU list parsing functions to `src/core/cpu.rs`
  - Implement `parse_cpu_list(input: &str) -> Result<Vec<u32>>` supporting:
    - Single CPU: `"0"` → `[0]`
    - List: `"0,2,4"` → `[0, 2, 4]`
    - Range: `"0-3"` → `[0, 1, 2, 3]`
    - Mixed: `"0-2,5,7-9"` → `[0, 1, 2, 5, 7, 8, 9]`
  - Implement `validate_cpu_ids(cpus: &[u32], max_cpu: u32) -> Result<()>` to check CPU IDs exist
  - Add comprehensive unit tests for all parsing patterns
  - Test error cases (invalid syntax, CPU out of range, duplicate CPUs)

  **Must NOT do**:
  - Don't add NUMA-aware CPU selection
  - Don't add CPU topology mapping (physical vs logical)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Well-defined parsing logic with clear test cases
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: Tasks 6, 7
  - **Blocked By**: None

  **References**:
  - `src/core/cpu.rs` (from Task 1) - Module location
  - Linux perf tool source: `tools/perf/util/cpumap.c` - Reference implementation
  - Rust `nom` parser combinator (optional) - If using parser library

  **Why Each Reference Matters**:
  - `src/core/cpu.rs`: Where to add parsing functions
  - perf tool source: Authoritative reference for syntax support
  - `nom` crate: Optional for robust parsing (can also use simple string parsing)

  **Acceptance Criteria**:
  - [ ] `parse_cpu_list()` implemented with all syntax support
  - [ ] `validate_cpu_ids()` implemented
  - [ ] Unit tests for: single, list, range, mixed, invalid syntax, out of range
  - [ ] `cargo test --lib cpu::tests::parse` passes
  - [ ] Error messages are clear and helpful

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Parse all CPU list formats
    Tool: Bash
    Preconditions: None
    Steps:
      1. cargo test test_parse_cpu_list -- --nocapture
      2. Assert all parsing tests pass
    Expected Result: All formats (single, list, range, mixed) parse correctly
    Failure Indicators: Any test fails
    Evidence: .sisyphus/evidence/task-02-parse-success.txt

  Scenario: Reject invalid CPU list syntax
    Tool: Bash
    Preconditions: None
    Steps:
      1. cargo test test_parse_cpu_list_invalid -- --nocapture
      2. Assert invalid syntax tests pass
    Expected Result: Invalid formats return appropriate errors
    Failure Indicators: Invalid input is accepted without error
    Evidence: .sisyphus/evidence/task-02-parse-error.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add CPU list parser with validation`
  - Files: `src/core/cpu.rs`
  - Pre-commit: `cargo test --lib cpu`

- [x] 3. Privilege Check Extension

  **What to do**:
  - Extend `src/core/privilege.rs` with system-wide profiling check
  - Add method `can_profile_system_wide(&self) -> bool` to `PrivilegeLevel`
  - Implement check: `perf_event_paranoid <= 0` OR has CAP_PERFMON/CAP_SYS_ADMIN
  - Add new `PerfError` variant for system-wide permission denied
  - Add unit tests for privilege checking logic
  - Update documentation in privilege module

  **Must NOT do**:
  - Don't change existing `check_privilege()` behavior
  - Don't add container/Cgroup awareness

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Extending existing well-structured privilege module
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4)
  - **Blocks**: Tasks 8, 10
  - **Blocked By**: None

  **References**:
  - `src/core/privilege.rs:120-144` - Existing privilege check implementation
  - `src/error.rs` - PerfError variants
  - Linux perf_event_open man page - perf_event_paranoid sysctl documentation

  **Why Each Reference Matters**:
  - `privilege.rs`: Pattern to follow for new method
  - `error.rs`: Where to add new error variant
  - Man page: Documents permission requirements for system-wide profiling

  **Acceptance Criteria**:
  - [ ] `can_profile_system_wide()` method added to `PrivilegeLevel`
  - [ ] New `PerfError::SystemWidePermissionDenied` variant added
  - [ ] Unit tests: `test_can_profile_system_wide_full()`, `test_can_profile_system_wide_limited()`
  - [ ] `cargo test --lib privilege` passes
  - [ ] Method returns correct boolean based on privilege level and paranoid setting

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: System-wide privilege check works correctly
    Tool: Bash
    Preconditions: None
    Steps:
      1. cargo test test_can_profile_system_wide -- --nocapture
      2. Assert tests pass with expected privilege levels
    Expected Result: Full privileges return true, Limited/None return false
    Failure Indicators: Incorrect privilege level determination
    Evidence: .sisyphus/evidence/task-03-privilege-check.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add system-wide profiling privilege check`
  - Files: `src/core/privilege.rs`, `src/error.rs`
  - Pre-commit: `cargo test --lib privilege`

- [x] 4. PerfConfig with_cpu() Builder

  **What to do**:
  - Extend `src/core/perf_event.rs` with CPU selection builders
  - Add `with_cpu(self, cpu: u32) -> Self` builder method
  - Add `with_all_cpus(self) -> Self` builder method (sets cpu = None for any CPU)
  - Update `create_counter()` to handle CPU selection:
    - If `cpu` is Some(value), use `builder.one_cpu(value)`
    - If `cpu` is None, use `builder.any_cpu()`
  - Add unit tests for CPU builder methods
  - Update PerfConfig documentation

  **Must NOT do**:
  - Don't change existing `with_pid()` behavior
  - Don't modify existing counter creation logic (extend, don't replace)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple builder pattern extension
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: Tasks 5, 8, 10
  - **Blocked By**: None

  **References**:
  - `src/core/perf_event.rs:6-13` - PerfConfig struct definition
  - `src/core/perf_event.rs:44-70` - create_counter() implementation
  - `src/core/perf_event.rs:20-30` - Existing with_pid() builder pattern
  - perf-event2 crate docs - Builder API documentation

  **Why Each Reference Matters**:
  - Lines 6-13: Struct to extend with cpu field (already exists but needs builder)
  - Lines 44-70: Where to add CPU selection logic
  - Lines 20-30: Pattern to follow for new builder methods
  - perf-event2 docs: API for one_cpu() and any_cpu()

  **Acceptance Criteria**:
  - [ ] `with_cpu()` and `with_all_cpus()` builder methods added
  - [ ] `create_counter()` uses correct CPU selection based on config
  - [ ] Unit tests: `test_perf_config_with_cpu()`, `test_perf_config_all_cpus()`
  - [ ] `cargo test --lib perf_event` passes
  - [ ] Existing tests still pass (no regression)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: CPU builder methods work correctly
    Tool: Bash
    Preconditions: None
    Steps:
      1. cargo test test_perf_config_with_cpu -- --nocapture
      2. cargo test test_perf_config_all_cpus -- --nocapture
      3. Assert both tests pass
    Expected Result: CPU config is set correctly in PerfConfig
    Failure Indicators: CPU field not set or wrong value
    Evidence: .sisyphus/evidence/task-04-config-builder.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add CPU selection builders to PerfConfig`
  - Files: `src/core/perf_event.rs`
  - Pre-commit: `cargo test --lib perf_event`

- [x] 5. RingBuffer CPU-Wide Creation

  **What to do**:
  - Extend `src/core/ringbuf.rs` with CPU-wide buffer creation
  - Add method `from_event_for_cpu(event: Event, cpu: u32, sample_period: u64, enable_on_exec: bool) -> Result<Self>`
  - Create counter with `pid = -1` (all processes) and specified CPU
  - Map ring buffer for reading samples
  - Add unit tests (may need mocking or #[ignore] for privilege requirements)
  - Update module documentation

  **Must NOT do**:
  - Don't change existing `from_event_for_pid()` behavior
  - Don't add threading (use sequential polling)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Extending existing ring buffer module with clear pattern
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 7)
  - **Blocks**: Task 10
  - **Blocked By**: Task 4 (PerfConfig needs CPU builder first)

  **References**:
  - `src/core/ringbuf.rs:46-65` - Existing from_event_for_pid() implementation
  - `src/core/perf_event.rs:44-70` - Counter creation pattern
  - perf-event2 crate docs - observe_pid() vs pid=-1 for system-wide
  - Linux perf_event_open man page - pid=-1 semantics for CPU-wide

  **Why Each Reference Matters**:
  - Lines 46-65: Pattern to follow for new method
  - Counter creation: How to set up event with CPU selection
  - perf-event2 docs: API for creating system-wide counter
  - Man page: Explains pid=-1 with cpu=N semantics

  **Acceptance Criteria**:
  - [ ] `from_event_for_cpu()` method added
  - [ ] Counter created with pid=-1 and specified CPU
  - [ ] Ring buffer mapped correctly for reading
  - [ ] Unit tests (marked #[ignore] if needs root)
  - [ ] `cargo test --lib ringbuf` passes
  - [ ] `cargo test --lib ringbuf -- --ignored` passes with root

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: CPU-wide ring buffer creation succeeds with privileges
    Tool: Bash
    Preconditions: Running with root or CAP_PERFMON
    Steps:
      1. sudo cargo test test_from_event_for_cpu -- --ignored --nocapture
      2. Assert test passes
    Expected Result: Ring buffer created successfully for CPU 0
    Failure Indicators: Test fails with permission error or panic
    Evidence: .sisyphus/evidence/task-05-ringbuf-cpu.txt

  Scenario: CPU-wide ring buffer fails without privileges
    Tool: Bash
    Preconditions: Running without sufficient privileges
    Steps:
      1. cargo test test_from_event_for_cpu_permission_denied -- --nocapture
      2. Assert test passes with expected error
    Expected Result: Appropriate permission error returned
    Failure Indicators: Test succeeds unexpectedly or wrong error
    Evidence: .sisyphus/evidence/task-05-ringbuf-permission.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add CPU-wide ring buffer creation`
  - Files: `src/core/ringbuf.rs`
  - Pre-commit: `cargo test --lib ringbuf`

- [x] 6. CLI Flags for Stat Command

  **What to do**:
  - Extend `src/cli.rs` with system-wide flags for stat command
  - Add `-a/--all-cpus` flag (boolean, enables system-wide mode)
  - Add `-C/--cpu <CPUS>` option (string, accepts CPU list: "0,2,4-6")
  - Add `--per-cpu` flag (boolean, shows per-CPU breakdown)
  - Update `StatArgs` struct with new fields
  - Add validation: `-a` and `-C` are mutually exclusive (return error if both specified)
  - Pass flags to stat command implementation
  - Update command documentation/help text

  **Must NOT do**:
  - Don't change existing stat flags behavior
  - Don't add new profiling modes (only system-wide)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Straightforward CLI extension following existing patterns
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 7)
  - **Blocks**: Tasks 8, 9
  - **Blocked By**: Tasks 1, 2 (CPU utilities needed for validation)

  **References**:
  - `src/cli.rs` - Existing CLI structure and clap derive patterns
  - `src/commands/stat.rs` - StatArgs struct and command entry point
  - Tasks 1, 2 - CPU detection and parsing utilities

  **Why Each Reference Matters**:
  - `cli.rs`: Pattern for adding flags with clap derive macros
  - `stat.rs`: Where to add new args fields and validation
  - Tasks 1, 2: Use CPU parsing for validation in CLI

  **Acceptance Criteria**:
  - [ ] `-a/--all-cpus` flag added to stat command
  - [ ] `-C/--cpu` option added with CPU list support
  - [ ] `--per-cpu` flag added
  - [ ] Mutual exclusivity validation between `-a` and `-C`
  - [ ] Help text updated for all new flags
  - [ ] `cargo run -- stat --help` shows new flags
  - [ ] Compilation succeeds with no warnings

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: CLI flags are recognized
    Tool: Bash
    Preconditions: Project builds successfully
    Steps:
      1. cargo run -- stat --help | grep -E '(--all-cpus|--cpu|--per-cpu)'
      2. Assert all three flags appear in help output
    Expected Result: All new flags shown in help text
    Failure Indicators: Flags not found in help output
    Evidence: .sisyphus/evidence/task-06-cli-help.txt

  Scenario: Mutual exclusivity validation works
    Tool: Bash
    Preconditions: Project builds successfully
    Steps:
      1. cargo run -- stat -a -C 0 sleep 1 2>&1
      2. Assert output contains error about conflicting flags
      3. Assert exit code != 0
    Expected Result: Error message displayed, command fails
    Failure Indicators: Command executes or wrong error
    Evidence: .sisyphus/evidence/task-06-cli-conflict.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): add system-wide flags to stat command`
  - Files: `src/cli.rs`, `src/commands/stat.rs`
  - Pre-commit: `cargo build`

- [x] 7. CLI Flags for Record Command

  **What to do**:
  - Extend `src/cli.rs` with system-wide flags for record command
  - Add `-a/--all-cpus` flag (boolean, enables system-wide mode)
  - Add `-C/--cpu <CPUS>` option (string, accepts CPU list)
  - Update `RecordArgs` struct with new fields
  - Add validation: `-a` and `-C` are mutually exclusive
  - Pass flags to record command implementation
  - Update command documentation/help text

  **Must NOT do**:
  - Don't add `--per-cpu` flag (not applicable to record)
  - Don't change existing record flags behavior

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Same pattern as Task 6, even simpler
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6)
  - **Blocks**: Task 10
  - **Blocked By**: Tasks 1, 2 (CPU utilities needed)

  **References**:
  - `src/cli.rs` - CLI structure (see Task 6 for pattern)
  - `src/commands/record.rs` - RecordArgs struct
  - Tasks 1, 2 - CPU utilities

  **Why Each Reference Matters**:
  - `cli.rs`: Same pattern as stat flags
  - `record.rs`: Where to add new args fields
  - Tasks 1, 2: CPU parsing utilities

  **Acceptance Criteria**:
  - [ ] `-a/--all-cpus` flag added to record command
  - [ ] `-C/--cpu` option added with CPU list support
  - [ ] Mutual exclusivity validation between `-a` and `-C`
  - [ ] Help text updated
  - [ ] `cargo run -- record --help` shows new flags
  - [ ] Compilation succeeds with no warnings

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: CLI flags are recognized for record
    Tool: Bash
    Preconditions: Project builds successfully
    Steps:
      1. cargo run -- record --help | grep -E '(--all-cpus|--cpu)'
      2. Assert both flags appear in help output
    Expected Result: Both new flags shown in help text
    Failure Indicators: Flags not found in help output
    Evidence: .sisyphus/evidence/task-07-record-cli-help.txt

  Scenario: Mutual exclusivity validation works for record
    Tool: Bash
    Preconditions: Project builds successfully
    Steps:
      1. cargo run -- record -a -C 0 sleep 1 2>&1
      2. Assert output contains error about conflicting flags
      3. Assert exit code != 0
    Expected Result: Error message displayed, command fails
    Failure Indicators: Command executes or wrong error
    Evidence: .sisyphus/evidence/task-07-record-cli-conflict.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): add system-wide flags to record command`
  - Files: `src/cli.rs`, `src/commands/record.rs`
  - Pre-commit: `cargo build`

- [x] 8. System-Wide Stat Implementation (Aggregated)

  **What to do**:
  - Modify `src/commands/stat.rs` to support system-wide mode
  - Detect system-wide mode from args (`-a` or `-C` specified)
  - Check privileges using `can_profile_system_wide()` (return clear error if insufficient)
  - Get CPU list: all CPUs for `-a`, parsed list for `-C`
  - Create counter for each selected CPU (one counter per event per CPU)
  - Use `PerfConfig::new().with_cpu(cpu)` for each CPU
  - Collect and aggregate counter values across all CPUs
  - Display aggregated results (single value per event)
  - Handle counter creation failures gracefully (CPU offline, etc.)

  **Must NOT do**:
  - Don't implement per-CPU breakdown yet (Task 9)
  - Don't change process-level stat behavior
  - Don't add event filtering or auto-selection

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core business logic requiring careful implementation and error handling
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on Wave 1 and 2 completion)
  - **Parallel Group**: Wave 3 (with Tasks 9, 10)
  - **Blocks**: Tasks 9, 11
  - **Blocked By**: Tasks 3, 4, 6 (privilege check, PerfConfig builder, CLI flags)

  **References**:
  - `src/commands/stat.rs:82-88` - Existing multi-counter pattern
  - `src/core/perf_event.rs:44-70` - Counter creation with CPU selection
  - `src/core/privilege.rs` - Privilege checking (from Task 3)
  - `src/core/cpu.rs` - CPU detection and parsing (from Tasks 1, 2)
  - Linux perf tool source - Aggregation logic reference

  **Why Each Reference Matters**:
  - Lines 82-88: Pattern for handling multiple counters
  - Counter creation: How to create per-CPU counters
  - Privilege checking: Must validate before attempting system-wide
  - CPU utilities: Get CPU list and validate IDs
  - perf tool: Reference for aggregation semantics

  **Acceptance Criteria**:
  - [ ] System-wide mode detected from CLI args
  - [ ] Privilege check performed before counter creation
  - [ ] Per-CPU counters created successfully
  - [ ] Counter values aggregated across all CPUs (summed)
  - [ ] Aggregated results displayed correctly
  - [ ] Error handling for: insufficient privileges, offline CPUs, counter failures
  - [ ] Process-level stat still works unchanged
  - [ ] Unit tests for aggregation logic
  - [ ] Manual test: `sudo perf-rs stat -a sleep 1` shows aggregated output

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: System-wide stat produces aggregated output
    Tool: Bash
    Preconditions: Running with root privileges
    Steps:
      1. sudo target/debug/perf-rs stat -a sleep 1
      2. Assert output contains event names and counts
      3. Assert counts are non-zero
      4. Assert output shows single value per event (not per-CPU)
    Expected Result: Aggregated performance counters displayed
    Failure Indicators: No output, per-CPU breakdown shown, or errors
    Evidence: .sisyphus/evidence/task-08-stat-aggregated.txt

  Scenario: System-wide stat fails without privileges
    Tool: Bash
    Preconditions: Running without sufficient privileges
    Steps:
      1. target/debug/perf-rs stat -a sleep 1 2>&1
      2. Assert output contains permission error
      3. Assert exit code != 0
    Expected Result: Clear error message about insufficient privileges
    Failure Indicators: Command executes or wrong error
    Evidence: .sisyphus/evidence/task-08-stat-permission.txt

  Scenario: Specific CPU selection works
    Tool: Bash
    Preconditions: Running with root privileges, system has CPU 0 and 1
    Steps:
      1. sudo target/debug/perf-rs stat -C 0,1 sleep 1
      2. Assert output shows results (aggregated for CPU 0 and 1)
      3. Assert no errors
    Expected Result: Counters from specified CPUs aggregated correctly
    Failure Indicators: Errors or no output
    Evidence: .sisyphus/evidence/task-08-stat-specific-cpu.txt
  ```

  **Commit**: YES
  - Message: `feat(stat): implement system-wide stat mode`
  - Files: `src/commands/stat.rs`
  - Pre-commit: `cargo test --lib commands::stat`

- [x] 9. System-Wide Stat Per-CPU Output

  **What to do**:
  - Extend `src/commands/stat.rs` to support `--per-cpu` output format
  - Detect `--per-cpu` flag from args
  - Modify output logic: instead of aggregating, display per-CPU values
  - Format as table (similar to `perf stat --per-cpu`):
    ```
    CPU    EVENT              COUNT
    0      cpu-cycles         1234567
    0      instructions       987654
    1      cpu-cycles         1345678
    1      instructions       876543
    ```
  - Sort output by CPU ID, then by event name
  - Handle missing data (CPU offline during measurement)
  - Update help text to describe `--per-cpu` format

  **Must NOT do**:
  - Don't change aggregated output format (Task 8)
  - Don't add JSON or other output formats
  - Don't change event ordering logic

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Output formatting with specific requirements
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 10)
  - **Blocks**: Task 11
  - **Blocked By**: Task 8 (needs aggregated stat first)

  **References**:
  - `src/commands/stat.rs` - Current output formatting (from Task 8)
  - Linux perf tool output: `perf stat --per-cpu` - Format reference
  - `src/core/cpu.rs` - CPU list for iteration

  **Why Each Reference Matters**:
  - `stat.rs`: Where to add per-CPU output formatting
  - perf tool: Authoritative reference for output format
  - CPU utilities: Iterate over CPUs for output

  **Acceptance Criteria**:
  - [ ] `--per-cpu` flag detected and handled
  - [ ] Per-CPU values displayed in table format
  - [ ] Output sorted by CPU ID, then event name
  - [ ] Table headers clear and aligned
  - [ ] Works with both `-a` and `-C` CPU selection
  - [ ] Manual test: `sudo perf-rs stat -a --per-cpu sleep 1` shows per-CPU table

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Per-CPU output format is correct
    Tool: Bash
    Preconditions: Running with root privileges
    Steps:
      1. sudo target/debug/perf-rs stat -a --per-cpu sleep 1
      2. Assert output contains "CPU" column header
      3. Assert each CPU has multiple event rows
      4. Assert values are per-CPU, not aggregated
      5. Assert CPUs are sorted numerically
    Expected Result: Table format with per-CPU breakdown
    Failure Indicators: Wrong format, missing CPU column, or aggregated values
    Evidence: .sisyphus/evidence/task-09-per-cpu-output.txt

  Scenario: Per-CPU works with specific CPU selection
    Tool: Bash
    Preconditions: Running with root privileges
    Steps:
      1. sudo target/debug/perf-rs stat -C 0,1 --per-cpu sleep 1
      2. Assert output shows only CPU 0 and 1 rows
      3. Assert no other CPUs appear
    Expected Result: Only specified CPUs shown in output
    Failure Indicators: Wrong CPUs shown or missing rows
    Evidence: .sisyphus/evidence/task-09-per-cpu-specific.txt
  ```

  **Commit**: YES
  - Message: `feat(stat): add per-CPU output format`
  - Files: `src/commands/stat.rs`
  - Pre-commit: `cargo test --lib commands::stat`

- [x] 10. System-Wide Record Implementation

  **What to do**:
  - Modify `src/commands/record.rs` to support system-wide mode
  - Detect system-wide mode from args (`-a` or `-C`)
  - Check privileges using `can_profile_system_wide()`
  - Get CPU list: all CPUs for `-a`, parsed list for `-C`
  - Create ring buffer for each selected CPU using `from_event_for_cpu()`
  - Use `PerfConfig::new().with_cpu(cpu)` for each CPU
  - Add PERF_SAMPLE_CPU to sample_type in event config (for CPU ID in samples)
  - Collect samples from all ring buffers (sequential polling)
  - Write samples to single perf.data file
  - Ensure perf.data format is compatible with standard `perf report`
  - Handle buffer overruns and signal interrupts (Ctrl+C)

  **Must NOT do**:
  - Don't create separate files per CPU (single perf.data)
  - Don't add threading (use sequential polling)
  - Don't change process-level record behavior

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex coordination of multiple ring buffers and file I/O
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 9)
  - **Blocks**: Tasks 12, 13
  - **Blocked By**: Tasks 3, 4, 5, 7 (privilege, PerfConfig, RingBuffer, CLI)

  **References**:
  - `src/commands/record.rs` - Current record implementation
  - `src/core/ringbuf.rs` - Ring buffer API (from Task 5)
  - `src/core/perf_event.rs` - Event config with PERF_SAMPLE_CPU flag
  - `src/core/perf_data.rs` - Perf.data file writing
  - Linux perf_event_open man page - PERF_SAMPLE_CPU flag documentation

  **Why Each Reference Matters**:
  - `record.rs`: Where to add system-wide logic
  - `ringbuf.rs`: CPU-wide buffer creation method
  - `perf_event.rs`: How to set sample_type with CPU ID
  - `perf_data.rs`: File writing format
  - Man page: Documents PERF_SAMPLE_CPU semantics

  **Acceptance Criteria**:
  - [ ] System-wide mode detected and privileges checked
  - [ ] Per-CPU ring buffers created successfully
  - [ ] PERF_SAMPLE_CPU added to sample_type
  - [ ] Samples collected from all buffers into single perf.data
  - [ ] Perf.data file is valid and non-empty
  - [ ] `perf report` can read the generated file
  - [ ] Ctrl+C stops recording cleanly
  - [ ] Manual test: `sudo perf-rs record -a -o test.data sleep 1` succeeds

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: System-wide record creates valid perf.data
    Tool: Bash
    Preconditions: Running with root privileges
    Steps:
      1. sudo target/debug/perf-rs record -a -o /tmp/test.data sleep 1
      2. Assert exit code is 0
      3. Assert /tmp/test.data exists and size > 0
      4. perf report -i /tmp/test.data --stdio | head -20
      5. Assert perf report shows samples
    Expected Result: Valid perf.data file created, readable by perf report
    Failure Indicators: File not created, empty file, or perf report errors
    Evidence: .sisyphus/evidence/task-10-record-system-wide.txt

  Scenario: Specific CPU record works
    Tool: Bash
    Preconditions: Running with root privileges
    Steps:
      1. sudo target/debug/perf-rs record -C 0 -o /tmp/test-cpu0.data sleep 1
      2. Assert /tmp/test-cpu0.data exists and size > 0
      3. perf report -i /tmp/test-cpu0.data --stdio | head -10
    Expected Result: Valid perf.data with samples from CPU 0 only
    Failure Indicators: File errors or perf report failures
    Evidence: .sisyphus/evidence/task-10-record-specific-cpu.txt

  Scenario: System-wide record fails without privileges
    Tool: Bash
    Preconditions: Running without sufficient privileges
    Steps:
      1. target/debug/perf-rs record -a -o /tmp/test.data sleep 1 2>&1
      2. Assert output contains permission error
      3. Assert exit code != 0
    Expected Result: Clear error message about insufficient privileges
    Failure Indicators: Command executes or wrong error
    Evidence: .sisyphus/evidence/task-10-record-permission.txt
  ```

  **Commit**: YES
  - Message: `feat(record): implement system-wide record mode`
  - Files: `src/commands/record.rs`
  - Pre-commit: `cargo test --lib commands::record`

- [x] 11. Integration Tests for Stat

  **What to do**:
  - Create `tests/integration_stat.rs` for system-wide stat integration tests
  - Add tests for all stat modes:
    - System-wide aggregated (`-a`)
    - System-wide per-CPU (`-a --per-cpu`)
    - Specific CPUs (`-C 0,1`)
    - Error cases (insufficient privileges, invalid CPUs)
  - Mark privilege-requiring tests with `#[ignore]` attribute
  - Add test for conflicting flags (`-a -C 0`)
  - Document how to run tests: `cargo test -- --ignored` for root tests
  - Verify output format and correctness

  **Must NOT do**:
  - Don't add unit tests (those are in task-specific files)
  - Don't test process-level stat (already tested)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Integration tests require careful setup and verification
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 12, 13, 14)
  - **Blocks**: None
  - **Blocked By**: Tasks 8, 9 (stat implementation complete)

  **References**:
  - `tests/` directory - Existing test structure (if any)
  - `src/commands/stat.rs` - Implementation to test
  - Rust integration testing patterns
  - Task 8, 9 - What to test

  **Why Each Reference Matters**:
  - `tests/`: Where to place integration tests
  - `stat.rs`: Implementation details for test design
  - Integration patterns: How to structure tests
  - Tasks 8, 9: Features to verify

  **Acceptance Criteria**:
  - [ ] `tests/integration_stat.rs` created
  - [ ] Tests for: aggregated, per-CPU, specific CPU modes
  - [ ] Tests for error cases
  - [ ] Privilege tests marked `#[ignore]`
  - [ ] `cargo test` passes (ignoring root tests)
  - [ ] `sudo cargo test -- --ignored` passes (with root)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Integration tests pass
    Tool: Bash
    Preconditions: Project builds successfully
    Steps:
      1. cargo test --test integration_stat
      2. Assert all non-ignored tests pass
    Expected Result: Basic tests pass without root
    Failure Indicators: Any test failures
    Evidence: .sisyphus/evidence/task-11-integration-tests-basic.txt

  Scenario: Integration tests pass with root
    Tool: Bash
    Preconditions: Running with root privileges
    Steps:
      1. sudo cargo test --test integration_stat -- --ignored
      2. Assert all tests pass
    Expected Result: All integration tests pass
    Failure Indicators: Any test failures
    Evidence: .sisyphus/evidence/task-11-integration-tests-root.txt
  ```

  **Commit**: YES
  - Message: `test: add integration tests for system-wide stat`
  - Files: `tests/integration_stat.rs`
  - Pre-commit: `cargo test`

- [x] 12. Integration Tests for Record

  **What to do**:
  - Create `tests/integration_record.rs` for system-wide record integration tests
  - Add tests for all record modes:
    - System-wide (`-a`)
    - Specific CPUs (`-C 0,1`)
    - Perf.data file validation
    - Error cases (insufficient privileges, invalid CPUs)
  - Mark privilege-requiring tests with `#[ignore]`
  - Test that perf.data files are readable by `perf report`
  - Document how to run tests with root

  **Must NOT do**:
  - Don't add unit tests
  - Don't test process-level record

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Integration tests with external tool compatibility
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 11, 13, 14)
  - **Blocks**: None
  - **Blocked By**: Task 10 (record implementation complete)

  **References**:
  - `tests/integration_stat.rs` - Pattern from Task 11
  - `src/commands/record.rs` - Implementation to test
  - Task 10 - What to test

  **Why Each Reference Matters**:
  - `integration_stat.rs`: Pattern to follow
  - `record.rs`: Implementation details
  - Task 10: Features to verify

  **Acceptance Criteria**:
  - [ ] `tests/integration_record.rs` created
  - [ ] Tests for: system-wide, specific CPU modes
  - [ ] Tests for perf.data validity
  - [ ] Tests for error cases
  - [ ] Privilege tests marked `#[ignore]`
  - [ ] `cargo test` passes
  - [ ] `sudo cargo test -- --ignored` passes with root

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Record integration tests pass
    Tool: Bash
    Preconditions: Project builds successfully
    Steps:
      1. cargo test --test integration_record
      2. Assert all non-ignored tests pass
    Expected Result: Basic tests pass without root
    Failure Indicators: Any test failures
    Evidence: .sisyphus/evidence/task-12-record-tests-basic.txt

  Scenario: Record integration tests pass with root
    Tool: Bash
    Preconditions: Running with root privileges, perf tool installed
    Steps:
      1. sudo cargo test --test integration_record -- --ignored
      2. Assert all tests pass including perf.data validation
    Expected Result: All tests pass, perf.data readable by perf report
    Failure Indicators: Any test failures or perf tool errors
    Evidence: .sisyphus/evidence/task-12-record-tests-root.txt
  ```

  **Commit**: YES
  - Message: `test: add integration tests for system-wide record`
  - Files: `tests/integration_record.rs`
  - Pre-commit: `cargo test`

- [x] 13. Perf.data Compatibility Verification

  **What to do**:
  - Create `tests/perf_data_compat.rs` for perf.data format validation
  - Generate perf.data files from system-wide recording
  - Validate files can be read by standard `perf report` tool
  - Check that CPU IDs are present in samples (PERF_SAMPLE_CPU)
  - Test with different CPU configurations (single CPU, multiple CPUs, all CPUs)
  - Verify sample data integrity
  - Document any format differences or limitations

  **Must NOT do**:
  - Don't test process-level perf.data
  - Don't create new perf.data format variant

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires understanding perf.data format and external tool testing
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 11, 12, 14)
  - **Blocks**: None
  - **Blocked By**: Task 10 (record implementation complete)

  **References**:
  - `src/core/perf_data.rs` - Perf.data writing implementation
  - Linux perf tool documentation - perf.data format specification
  - `perf report` command - External tool for validation

  **Why Each Reference Matters**:
  - `perf_data.rs`: How perf.data is written
  - perf docs: Format specification
  - `perf report`: Compatibility target

  **Acceptance Criteria**:
  - [ ] `tests/perf_data_compat.rs` created
  - [ ] Tests generate perf.data from system-wide recording
  - [ ] Tests validate files with `perf report`
  - [ ] Tests verify CPU IDs in samples
  - [ ] Tests marked `#[ignore]` (require root and perf tool)
  - [ ] `sudo cargo test --test perf_data_compat -- --ignored` passes

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Perf.data is compatible with standard perf tool
    Tool: Bash
    Preconditions: Running with root, perf tool installed
    Steps:
      1. sudo cargo test --test perf_data_compat -- --ignored --nocapture
      2. Assert all compatibility tests pass
      3. Assert perf report successfully reads generated files
    Expected Result: All perf.data files are valid and readable
    Failure Indicators: perf report errors or test failures
    Evidence: .sisyphus/evidence/task-13-perf-data-compat.txt
  ```

  **Commit**: YES
  - Message: `test: verify perf.data compatibility`
  - Files: `tests/perf_data_compat.rs`
  - Pre-commit: `cargo test`

- [x] 14. Error Handling and Edge Cases

  **What to do**:
  - Review and enhance error handling across all system-wide profiling code
  - Handle edge cases:
    - CPU goes offline during profiling
    - CPU count changes during profiling (hotplug)
    - Invalid CPU IDs in `-C` list
    - Permission denied errors (clear messages)
    - Counter creation failures for some CPUs (partial success)
    - Ring buffer overruns
    - Signal handling (Ctrl+C with multiple buffers)
  - Ensure error messages are user-friendly and actionable
  - Add appropriate logging for debugging
  - Update error types if needed

  **Must NOT do**:
  - Don't add retry logic (let user retry)
  - Don't add automatic fallback modes

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires thorough review and good judgment for error messages
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 11, 12, 13)
  - **Blocks**: None
  - **Blocked By**: Tasks 8-10 (all implementations complete)

  **References**:
  - `src/error.rs` - PerfError variants
  - `src/commands/stat.rs` - Stat error handling (from Task 8)
  - `src/commands/record.rs` - Record error handling (from Task 10)
  - Rust error handling best practices

  **Why Each Reference Matters**:
  - `error.rs`: Error types to use
  - Command implementations: Where to add error handling
  - Best practices: Patterns for good error messages

  **Acceptance Criteria**:
  - [ ] All edge cases handled gracefully
  - [ ] Error messages are clear and actionable
  - [ ] No panics in production code (use Result)
  - [ ] Partial failures handled correctly (some CPUs fail)
  - [ ] Signal handling cleans up resources
  - [ ] Manual testing of error scenarios
  - [ ] `cargo clippy` passes with no warnings

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Invalid CPU ID produces clear error
    Tool: Bash
    Preconditions: System has N CPUs (N < 999)
    Steps:
      1. sudo target/debug/perf-rs stat -C 999 sleep 1 2>&1
      2. Assert error message mentions CPU 999 not found
      3. Assert error message mentions actual CPU count
      4. Assert exit code != 0
    Expected Result: Clear error about invalid CPU
    Failure Indicators: Vague error or panic
    Evidence: .sisyphus/evidence/task-14-invalid-cpu.txt

  Scenario: Conflicting flags produce clear error
    Tool: Bash
    Preconditions: Project builds
    Steps:
      1. sudo target/debug/perf-rs stat -a -C 0 sleep 1 2>&1
      2. Assert error message mentions conflicting flags
      3. Assert exit code != 0
    Expected Result: Clear error about flag conflict
    Failure Indicators: Wrong error or panic
    Evidence: .sisyphus/evidence/task-14-conflicting-flags.txt

  Scenario: Insufficient privileges produce clear error
    Tool: Bash
    Preconditions: Running without sufficient privileges
    Steps:
      1. target/debug/perf-rs stat -a sleep 1 2>&1
      2. Assert error message explains privilege requirements
      3. Assert exit code != 0
    Expected Result: Clear error about permissions needed
    Failure Indicators: Vague error or crash
    Evidence: .sisyphus/evidence/task-14-permission-error.txt
  ```

  **Commit**: YES
  - Message: `fix: improve error handling and edge cases`
  - Files: `src/commands/*.rs`, `src/error.rs`
  - Pre-commit: `cargo test && cargo clippy`

---

## Final Verification Wave (MANDATORY)

> 4 review agents run in PARALLEL. ALL must APPROVE. Rejection → fix → re-run.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists. For each "Must NOT Have": search codebase for forbidden patterns. Check evidence files exist. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy -- -D warnings` + `cargo fmt -- --check` + `cargo test`. Review all changed files for: `unwrap()` in production, `as any`, empty catches, commented code. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Clippy [PASS/FAIL] | Fmt [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from EVERY task. Test cross-command integration (stat and record both work with -a flag). Test error cases: insufficient privileges, invalid CPUs, conflicting flags. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Errors [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — everything in spec was built, nothing beyond spec. Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **1**: `feat(core): add CPU detection utility` — src/core/cpu.rs, src/core/mod.rs
- **2**: `feat(core): add CPU list parser with validation` — src/core/cpu.rs
- **3**: `feat(core): add system-wide profiling privilege check` — src/core/privilege.rs, src/error.rs
- **4**: `feat(core): add CPU selection builders to PerfConfig` — src/core/perf_event.rs
- **5**: `feat(core): add CPU-wide ring buffer creation` — src/core/ringbuf.rs
- **6**: `feat(cli): add system-wide flags to stat command` — src/cli.rs, src/commands/stat.rs
- **7**: `feat(cli): add system-wide flags to record command` — src/cli.rs, src/commands/record.rs
- **8**: `feat(stat): implement system-wide stat mode` — src/commands/stat.rs
- **9**: `feat(stat): add per-CPU output format` — src/commands/stat.rs
- **10**: `feat(record): implement system-wide record mode` — src/commands/record.rs
- **11**: `test: add integration tests for system-wide stat` — tests/integration_stat.rs
- **12**: `test: add integration tests for system-wide record` — tests/integration_record.rs
- **13**: `test: verify perf.data compatibility` — tests/perf_data_compat.rs
- **14**: `fix: improve error handling and edge cases` — src/commands/*.rs

---

## Success Criteria

### Verification Commands
```bash
# Build and format check
cargo build && cargo fmt -- --check && cargo clippy -- -D warnings

# Unit tests
cargo test

# Integration tests (require root)
sudo cargo test -- --ignored

# System-wide stat (aggregated)
sudo target/debug/perf-rs stat -a sleep 1
# Expected: Output shows all events with aggregated counts

# System-wide stat (per-CPU)
sudo target/debug/perf-rs stat -a --per-cpu sleep 1
# Expected: Output shows per-CPU breakdown in table format

# Specific CPUs
sudo target/debug/perf-rs stat -C 0,1 sleep 1
# Expected: Output shows data for CPU 0 and 1 only

# System-wide record
sudo target/debug/perf-rs record -a -o test.data sleep 1
# Expected: test.data created with non-zero size

# Perf.data compatibility
perf report -i test.data
# Expected: Standard perf tool can read the file

# Error case: insufficient privileges
target/debug/perf-rs stat -a sleep 1
# Expected: Error message about insufficient privileges
# Expected: Exit code != 0

# Error case: invalid CPU
sudo target/debug/perf-rs stat -C 999 sleep 1
# Expected: Error "CPU 999 not found (system has N CPUs)"
# Expected: Exit code != 0

# Error case: conflicting flags
sudo target/debug/perf-rs stat -a -C 0 sleep 1
# Expected: Error "Cannot use both --all-cpus and --cpu together"
# Expected: Exit code != 0
```

### Final Checklist
- [ ] All "Must Have" features implemented and tested
- [ ] All "Must NOT Have" constraints verified (no forbidden patterns in codebase)
- [ ] All unit tests pass (`cargo test`)
- [ ] All integration tests pass with root (`sudo cargo test -- --ignored`)
- [ ] System-wide stat works correctly (aggregated and per-CPU modes)
- [ ] System-wide record works correctly (single perf.data file)
- [ ] Perf.data files are compatible with standard `perf report`
- [ ] Error messages are clear and helpful
- [ ] Code passes `cargo clippy` with no warnings
- [ ] Code formatted with `cargo fmt`
- [ ] No regressions in existing functionality