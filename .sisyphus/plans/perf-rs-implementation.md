# perf-rs: Linux Performance Monitoring Tool

## TL;DR

> **Quick Summary**: Build a production-quality Rust implementation of core Linux perf functionality (stat, record, report, script, list) with multi-architecture support (x86_64, arm64, riscv64) and proper privilege management.
>
> **Deliverables**:
> - Functional perf-rs CLI tool with 5 subcommands
> - Multi-architecture support for performance event monitoring
> - perf.data file format support (read/write)
> - Symbol resolution for user and kernel code
> - Comprehensive error handling and privilege management
>
> **Estimated Effort**: Large (3-4 months for MVP)
> **Parallel Execution**: YES - 3 waves + final verification
> **Critical Path**: Project setup → perf list → perf stat → perf record → perf report → Integration

---

## Context

### Original Request
Create a tool named perf-rs in Rust which implements core functions of Linux perf, with commits after each task completion and proper .gitignore management.

### Interview Summary
**Key Discussions**:
- **Core features**: perf stat, perf record + report, perf script, perf list
- **Scope**: MVP - Core subset with clean architecture, extensible for future
- **Architectures**: x86_64, arm64, riscv64
- **Test strategy**: Tests after implementation
- **Kernel version**: Linux 5.0+ minimum
- **Implementation order**: perf list → perf stat → perf record → perf report → perf script
- **Git workflow**: Initialize repo first, commit after each task

**Research Findings**:
- **perf-event2 crate**: Safe, high-level API for perf_event_open syscall (primary choice)
- **Architecture support**: Requires trait-based abstraction for PMU events
- **Privilege management**: Check perf_event_paranoid, implement graceful degradation
- **Symbol resolution**: Use gimli for DWARF parsing, addr2line for higher-level API
- **perf.data format**: Use linux-perf-data crate initially, custom parser if needed
- **Ring buffer management**: Critical for sampling, needs overflow detection

### Metis Review
**Identified Gaps** (addressed):
- **perf.data format strategy**: Use linux-perf-data crate for parsing (MITIGATED)
- **Privilege management**: Implement check at startup, graceful degradation (MITIGATED)
- **Architecture event discovery**: Parse /sys/bus/event_source/devices/ (MITIGATED)
- **Symbol resolution scope**: Basic + DWARF for MVP, defer JIT support (MITIGATED)
- **Call chain unwinding**: Frame pointers for MVP, add DWARF later (MITIGATED)
- **Large dataset handling**: Memory-mapped files with lazy loading (MITIGATED)

---

## Work Objectives

### Core Objective
Implement a production-quality performance monitoring tool in Rust that provides core Linux perf functionality with multi-architecture support, proper error handling, and privilege management.

### Concrete Deliverables
- Git repository with proper .gitignore for Rust projects
- perf-rs binary with 5 subcommands: list, stat, record, report, script
- Multi-architecture support (x86_64, arm64, riscv64)
- perf.data file format support (read/write)
- Symbol resolution (ELF + DWARF)
- Comprehensive error handling and diagnostics
- Test suite validating core functionality

### Definition of Done
- [ ] All subcommands produce output comparable to standard perf tool
- [ ] Works on x86_64, arm64, riscv64 architectures
- [ ] Handles permission failures gracefully with clear error messages
- [ ] Git repository initialized with proper .gitignore
- [ ] All code committed after each task completion
- [ ] Tests pass on all supported architectures
- [ ] cargo build --release succeeds without warnings
- [ ] cargo clippy passes with no errors

### Must Have
- perf list: Enumerate available hardware and software events
- perf stat: Count events for specified commands/processes
- perf record: Sample-based profiling with call chains
- perf report: Analyze perf.data files, display hotspots
- perf script: Dump trace data in readable format
- Multi-architecture support with trait-based abstraction
- Privilege checking at startup (perf_event_paranoid)
- Graceful degradation for permission failures
- Git repository with commits after each task
- Proper .gitignore for Rust projects

### Must NOT Have (Guardrails from Metis)
- **NO** perf top (real-time TUI) in Phase 1 - defer to future work
- **NO** eBPF features before core functionality is stable
- **NO** Intel PT / ARM ETM support - advanced hardware tracing out of scope
- **NO** over-engineered symbol resolution - start with basic ELF + DWARF
- **NO** custom perf.data parser unless linux-perf-data proves insufficient
- **NO** features not in original perf tool without explicit request
- **NO** skipping error handling for edge cases (permissions, missing symbols)
- **NO** assuming identical PMU capabilities across architectures
- **NO** excessive comments or generic names (data/result/item/temp) - avoid AI slop
- **NO** premature abstraction - implement concrete cases first

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.
> Acceptance criteria requiring "user manually tests/confirms" are FORBIDDEN.

### Test Decision
- **Infrastructure exists**: NO (will create during implementation)
- **Automated tests**: Tests after implementation
- **Framework**: cargo test (built-in Rust test framework)
- **Test approach**: Unit tests for core logic, integration tests for CLI commands

### QA Policy
Every task MUST include agent-executed QA scenarios (see TODO template below).
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **CLI commands**: Use Bash — Run command, capture output, assert exit code and output content
- **File operations**: Use Bash — Test file existence, content, permissions
- **Performance tests**: Use Bash — Run workloads, verify event counting works
- **Error scenarios**: Use Bash — Trigger error conditions, verify graceful handling

---

## Execution Strategy

### Parallel Execution Waves

> Maximize throughput by grouping independent tasks into parallel waves.
> Each wave completes before the next begins.
> Target: 5-8 tasks per wave. Fewer than 3 per wave (except final) = under-splitting.

```
Wave 1 (Foundation - can start immediately):
├── Task 1: Git repository + .gitignore setup [quick]
├── Task 2: Cargo.toml with dependencies [quick]
├── Task 3: Error type definitions [quick]
├── Task 4: CLI structure with clap [quick]
├── Task 5: Architecture abstraction traits [quick]
├── Task 6: Privilege checking module [quick]
└── Task 7: Basic perf_event wrapper [quick]

Wave 2 (Core features - after Wave 1):
├── Task 8: perf list implementation [unspecified-high]
├── Task 9: perf stat - event counting [unspecified-high]
├── Task 10: perf stat - multi-event groups [unspecified-high]
├── Task 11: perf record - ring buffer setup [unspecified-high]
├── Task 12: perf record - sampling and file writing [deep]
├── Task 13: perf.data file format support [unspecified-high]
└── Task 14: Symbol resolution module [deep]

Wave 3 (Advanced features - after Wave 2):
├── Task 15: perf report - file parsing [unspecified-high]
├── Task 16: perf report - hotspot analysis [deep]
├── Task 17: perf report - symbol integration [unspecified-high]
├── Task 18: perf script implementation [unspecified-high]
├── Task 19: Multi-architecture event discovery [unspecified-high]
├── Task 20: Error handling and diagnostics [unspecified-high]
└── Task 21: Documentation and examples [writing]

Wave FINAL (Verification - after ALL implementation):
├── Task F1: Plan compliance audit [oracle]
├── Task F2: Code quality review [unspecified-high]
├── Task F3: Integration testing [deep]
└── Task F4: Scope fidelity check [deep]

Critical Path: Task 1 → Task 2 → Task 7 → Task 8 → Task 9 → Task 11 → Task 12 → Task 13 → Task 15 → Task 16 → F1-F4
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 7 (Waves 1 & 2)
```

### Dependency Matrix (abbreviated — show ALL tasks in your generated plan)

- **1**: — — 2-21, F1-F4
- **2**: 1 — 3-21, F1-F4
- **3**: 2 — 4-21, F1-F4
- **4**: 2, 3 — 8, 9, 15, 18
- **5**: 2, 3 — 8, 19
- **6**: 2, 3 — 9, 11
- **7**: 2, 3, 5, 6 — 8-14, 19
- **8**: 4, 5, 7 — F1-F4
- **9**: 4, 6, 7 — F1-F4
- **10**: 9 — F1-F4
- **11**: 6, 7 — 12, 13
- **12**: 11 — 15, 18
- **13**: 11, 12 — 15, 18
- **14**: 2, 3 — 16, 17
- **15**: 4, 12, 13 — 16, 18
- **16**: 14, 15 — F1-F4
- **17**: 14, 15, 16 — F1-F4
- **18**: 4, 12, 13, 15 — F1-F4
- **19**: 5, 7 — F1-F4
- **20**: 3, 6 — F1-F4
- **21**: 8-20 — F1-F4
- **F1-F4**: 1-21 — —

> This is abbreviated for reference. YOUR generated plan must include the FULL matrix for ALL tasks.

### Agent Dispatch Summary

- **Wave 1**: **7 tasks** — T1-T4, T6 → `quick`, T5, T7 → `quick`
- **Wave 2**: **7 tasks** — T8-T11 → `unspecified-high`, T12, T14 → `deep`, T13 → `unspecified-high`
- **Wave 3**: **7 tasks** — T15, T17, T19, T20 → `unspecified-high`, T16, T18 → `deep`, T21 → `writing`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task MUST have: Recommended Agent Profile + Parallelization info + QA Scenarios.
> **A task WITHOUT QA Scenarios is INCOMPLETE. No exceptions.**

- [x] 1. Git repository + .gitignore setup

  **What to do**:
  - Initialize git repository in project root
  - Create comprehensive .gitignore for Rust projects
  - Include patterns: target/, *.rs.bk, *.swp, .DS_Store, perf.data, *.perf
  - Commit initial state

  **Must NOT do**:
  - DO NOT add binary files or build artifacts to git
  - DO NOT include IDE-specific files (.idea/, .vscode/, etc.) unless requested
  - DO NOT add .env or credential files

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple file creation and git init, well-defined task
  - **Skills**: [`git-master`]
    - `git-master`: Git repository initialization best practices

  **Parallelization**:
  - **Can Run In Parallel**: NO - must be first task
  - **Parallel Group**: Sequential (foundation for all other tasks)
  - **Blocks**: Tasks 2-21, F1-F4
  - **Blocked By**: None

  **References** (CRITICAL - Be Exhaustive):
  **Pattern References**:
  - Standard Rust .gitignore template from github.com/github/gitignore

  **Acceptance Criteria**:
  - [ ] Git repository initialized (.git directory exists)
  - [ ] .gitignore file created with Rust patterns
  - [ ] Initial commit created with message "chore: initialize git repository with .gitignore"

  **QA Scenarios (MANDATORY)**:
  ```
  Scenario: Git repository properly initialized
    Tool: Bash
    Preconditions: Project directory exists
    Steps:
      1. test -d .git && echo "PASS: Git repository exists"
      2. git status | grep -q "On branch" && echo "PASS: Git status works"
    Expected Result: Both checks pass
    Evidence: .sisyphus/evidence/task-01-git-init.txt

  Scenario: .gitignore has Rust patterns
    Tool: Bash
    Preconditions: .gitignore file exists
    Steps:
      1. grep -q "target/" .gitignore && echo "PASS: target/ ignored"
      2. grep -q "*.rs.bk" .gitignore && echo "PASS: backup files ignored"
      3. grep -q "perf.data" .gitignore && echo "PASS: perf.data ignored"
    Expected Result: All patterns present
    Evidence: .sisyphus/evidence/task-01-gitignore.txt
  ```

  **Commit**: YES
  - Message: `chore: initialize git repository with .gitignore`
  - Files: .gitignore
  - Pre-commit: None

- [x] 2. Cargo.toml with dependencies

  **What to do**:
  - Create Cargo.toml with project metadata
  - Add dependencies: perf-event2, clap (derive features), thiserror, anyhow
  - Add dependencies: serde (derive), toml, config
  - Add dependencies: nix (process/signal features), libc, procfs
  - Add dependencies: gimli, addr2line (object feature)
  - Add dev-dependencies: tempfile for testing
  - Set edition = "2021" and rust-version = "1.70"

  **Must NOT do**:
  - DO NOT add unused dependencies
  - DO NOT pin exact versions (use semver ranges)
  - DO NOT add optional dependencies unless needed

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Standard Cargo.toml creation, well-defined dependencies
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: NO - depends on Task 1
  - **Parallel Group**: Sequential (with Task 1)
  - **Blocks**: Tasks 3-21, F1-F4
  - **Blocked By**: Task 1

  **References**:
  **External References**:
  - perf-event2 docs: https://docs.rs/perf-event2/
  - clap docs: https://docs.rs/clap/
  - gimli docs: https://docs.rs/gimli/

  **Acceptance Criteria**:
  - [ ] Cargo.toml created with all required dependencies
  - [ ] cargo check succeeds without errors
  - [ ] Dependencies compile successfully

  **QA Scenarios**:
  ```
  Scenario: Cargo.toml has required dependencies
    Tool: Bash
    Preconditions: Cargo.toml exists
    Steps:
      1. grep -q 'perf-event2' Cargo.toml && echo "PASS: perf-event2 present"
      2. grep -q 'clap' Cargo.toml && echo "PASS: clap present"
      3. grep -q 'gimli' Cargo.toml && echo "PASS: gimli present"
      4. grep -q 'thiserror' Cargo.toml && echo "PASS: thiserror present"
    Expected Result: All dependencies present
    Evidence: .sisyphus/evidence/task-02-cargo-deps.txt

  Scenario: Project compiles
    Tool: Bash
    Preconditions: Cargo.toml exists
    Steps:
      1. cargo check 2>&1 | tee /tmp/cargo-check.log
      2. test ${PIPESTATUS[0]} -eq 0 && echo "PASS: cargo check succeeds"
    Expected Result: Compilation succeeds
    Evidence: .sisyphus/evidence/task-02-compile.txt
  ```

  **Commit**: YES
  - Message: `chore: add Cargo.toml with dependencies`
  - Files: Cargo.toml, Cargo.lock
  - Pre-commit: cargo check

- [x] 3. Error type definitions

  **What to do**:
  - Create src/error.rs with comprehensive error types
  - Use thiserror for library errors
  - Define error variants: Permission, ProcessNotFound, CounterSetup, CounterEnable, etc.
  - Add context fields for each error (pid, event name, paths, etc.)
  - Implement Display and Error traits automatically via thiserror

  **Must NOT do**:
  - DO NOT use unwrap() in error definitions
  - DO NOT create overly generic error types
  - DO NOT skip error context information

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Standard error type definition, well-established pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO - depends on Task 2
  - **Parallel Group**: Sequential (with Task 2)
  - **Blocks**: Tasks 4-21, F1-F4
  - **Blocked By**: Task 2

  **References**:
  **Pattern References**:
  - thiserror crate examples for error definitions
  - Research findings show specific error types needed: ProcessAttach, ProcessNotFound, CounterSetup, PermissionDenied

  **Acceptance Criteria**:
  - [ ] src/error.rs created with PerfError enum
  - [ ] All error variants have context fields
  - [ ] thiserror derives implemented correctly
  - [ ] cargo check passes

  **QA Scenarios**:
  ```
  Scenario: Error types compile
    Tool: Bash
    Preconditions: src/error.rs exists
    Steps:
      1. cargo check 2>&1 | grep -q "error" && echo "FAIL" || echo "PASS: Error types compile"
    Expected Result: Compilation succeeds
    Evidence: .sisyphus/evidence/task-03-error-compile.txt

  Scenario: Error types have thiserror derives
    Tool: Bash
    Preconditions: src/error.rs exists
    Steps:
      1. grep -q "#\[derive.*Error" src/error.rs && echo "PASS: Error derive present"
      2. grep -q "#\[error" src/error.rs && echo "PASS: Error attributes present"
    Expected Result: thiserror attributes present
    Evidence: .sisyphus/evidence/task-03-error-derive.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add error types`
  - Files: src/error.rs
  - Pre-commit: cargo check

- [x] 4. CLI structure with clap

  **What to do**:
  - Create src/cli.rs with clap derive structures
  - Define Cli struct with subcommands enum
  - Define subcommands: List, Stat, Record, Report, Script
  - Add arguments for each subcommand (pid, output, input, event, verbose, etc.)
  - Create src/main.rs with basic structure and argument parsing
  - Use anyhow for application error handling

  **Must NOT do**:
  - DO NOT implement subcommand logic yet (separate tasks)
  - DO NOT add unnecessary CLI options
  - DO NOT skip help text generation

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Standard CLI definition with clap, well-established pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO - depends on Task 3
  - **Parallel Group**: Sequential
  - **Blocks**: Tasks 8, 9, 15, 18 (subcommand implementations)
  - **Blocked By**: Tasks 2, 3

  **References**:
  **Pattern References**:
  - clap derive examples for subcommand structure
  - Research findings show similar tools like bottom use this pattern

  **External References**:
  - clap documentation: https://docs.rs/clap/

  **Acceptance Criteria**:
  - [ ] src/cli.rs created with Cli and Commands structs
  - [ ] src/main.rs created with basic structure
  - [ ] cargo run -- --help shows usage
  - [ ] cargo check passes

  **QA Scenarios**:
  ```
  Scenario: CLI help works
    Tool: Bash
    Preconditions: src/cli.rs and src/main.rs exist
    Steps:
      1. cargo run -- --help 2>&1 | grep -q "perf-rs" && echo "PASS: Help text shows"
      2. cargo run -- --help 2>&1 | grep -q "stat" && echo "PASS: stat subcommand listed"
      3. cargo run -- --help 2>&1 | grep -q "record" && echo "PASS: record subcommand listed"
    Expected Result: Help text displays correctly
    Evidence: .sisyphus/evidence/task-04-cli-help.txt

  Scenario: Subcommand help works
    Tool: Bash
    Preconditions: CLI structure exists
    Steps:
      1. cargo run -- list --help 2>&1 | grep -q "List" && echo "PASS: list help works"
      2. cargo run -- stat --help 2>&1 | grep -q "pid" && echo "PASS: stat help shows pid option"
    Expected Result: Subcommand help displays
    Evidence: .sisyphus/evidence/task-04-subcommand-help.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): add CLI structure with clap`
  - Files: src/cli.rs, src/main.rs
  - Pre-commit: cargo check

- [x] 5. Architecture abstraction traits

  **What to do**:
  - Create src/arch/mod.rs with architecture trait definitions
  - Define PmuEvent trait for architecture-specific PMU events
  - Define methods: get_hardware_events(), get_cache_events(), get_raw_events()
  - Create placeholder implementations for x86_64, arm64, riscv64
  - Use cfg attributes to conditionally compile architecture-specific code

  **Must NOT do**:
  - DO NOT implement detailed event enumeration yet (Task 19)
  - DO NOT assume all architectures have identical capabilities
  - DO NOT hardcode events without architecture checks

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Trait definition, standard Rust pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - independent of Tasks 3, 4
  - **Parallel Group**: Wave 1 (with Tasks 3, 4, 6, 7)
  - **Blocks**: Tasks 8, 19
  - **Blocked By**: Tasks 1, 2

  **References**:
  **Pattern References**:
  - Research findings show architecture abstraction is critical
  - Metis review identified architecture-specific challenges

  **Acceptance Criteria**:
  - [ ] src/arch/mod.rs created with trait definitions
  - [ ] Placeholder modules for x86_64, arm64, riscv64 created
  - [ ] cargo check passes

  **QA Scenarios**:
  ```
  Scenario: Architecture traits compile
    Tool: Bash
    Preconditions: src/arch/mod.rs exists
    Steps:
      1. cargo check 2>&1 | grep -q "error" && echo "FAIL" || echo "PASS: Architecture traits compile"
    Expected Result: Compilation succeeds
    Evidence: .sisyphus/evidence/task-05-arch-compile.txt

  Scenario: Architecture modules exist
    Tool: Bash
    Preconditions: src/arch/ directory exists
    Steps:
      1. test -f src/arch/mod.rs && echo "PASS: mod.rs exists"
      2. grep -q "trait PmuEvent" src/arch/mod.rs && echo "PASS: PmuEvent trait defined"
    Expected Result: Architecture structure present
    Evidence: .sisyphus/evidence/task-05-arch-modules.txt
  ```

  **Commit**: YES
  - Message: `feat(arch): add architecture abstraction traits`
  - Files: src/arch/mod.rs
  - Pre-commit: cargo check

- [x] 6. Privilege checking module

  **What to do**:
  - Create src/core/privilege.rs
  - Read /proc/sys/kernel/perf_event_paranoid
  - Implement check_privilege() function that returns privilege level
  - Define PrivilegeLevel enum: Full, Limited, None
  - Implement graceful degradation suggestions based on privilege level
  - Add capability checking (CAP_PERFMON, CAP_SYS_ADMIN)

  **Must NOT do**:
  - DO NOT fail silently when permissions insufficient
  - DO NOT assume user has root privileges
  - DO NOT skip privilege checks before perf operations

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Well-defined system file parsing and logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - independent of Tasks 3, 4, 5
  - **Parallel Group**: Wave 1 (with Tasks 3, 4, 5, 7)
  - **Blocks**: Tasks 9, 11
  - **Blocked By**: Tasks 1, 2

  **References**:
  **Pattern References**:
  - Research findings show perf_event_paranoid is critical
  - Metis review identified privilege management as critical requirement
  - Values: -1 (allow all), 0 (kernel profiling), 1 (normal), 2 (restricted)

  **External References**:
  - Linux perf documentation: https://kernel.org/doc/html/latest/admin-guide/perf-security.html

  **Acceptance Criteria**:
  - [ ] src/core/privilege.rs created
  - [ ] check_privilege() function implemented
  - [ ] PrivilegeLevel enum defined
  - [ ] cargo check passes

  **QA Scenarios**:
  ```
  Scenario: Privilege check reads perf_event_paranoid
    Tool: Bash
    Preconditions: src/core/privilege.rs exists
    Steps:
      1. cargo test --lib test_privilege_check 2>&1 | tee /tmp/priv-test.log
      2. grep -q "test result: ok" /tmp/priv-test.log && echo "PASS: Privilege check works"
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-06-privilege-test.txt

  Scenario: Graceful degradation works
    Tool: Bash
    Preconditions: privilege module implemented
    Steps:
      1. cargo run -- stat -- echo "test" 2>&1 | tee /tmp/priv-run.log
      2. grep -q "permission\|paranoid\|privilege\|success" /tmp/priv-run.log && echo "PASS: Privilege handled"
    Expected Result: Appropriate message shown based on privilege level
    Evidence: .sisyphus/evidence/task-06-privilege-run.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add privilege checking`
  - Files: src/core/privilege.rs, src/core/mod.rs
  - Pre-commit: cargo check

- [x] 7. Basic perf_event wrapper

  **What to do**:
  - Create src/core/perf_event.rs
  - Wrap perf-event2 crate's Builder API
  - Implement helper functions: create_counter(), create_group()
  - Add configuration struct PerfConfig with common options
  - Implement enable/disable/reset wrappers
  - Add error handling for counter operations

  **Must NOT do**:
  - DO NOT reimplement perf-event2 functionality (use the crate)
  - DO NOT expose unsafe APIs without safety documentation
  - DO NOT skip error handling for syscall failures

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: API wrapper creation, straightforward
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends only on Tasks 5, 6
  - **Parallel Group**: Wave 1 (with Tasks 3-6)
  - **Blocks**: Tasks 8-14, 19
  - **Blocked By**: Tasks 1, 2, 5, 6

  **References**:
  **API References**:
  - perf-event2 crate documentation: https://docs.rs/perf-event2/
  - perf_event::Builder pattern

  **Pattern References**:
  - Research shows perf-event2 provides safe abstractions
  - Counter and Group APIs already well-designed

  **Acceptance Criteria**:
  - [ ] src/core/perf_event.rs created
  - [ ] Wrapper functions for common operations implemented
  - [ ] cargo check passes

  **QA Scenarios**:
  ```
  Scenario: perf_event wrapper compiles
    Tool: Bash
    Preconditions: src/core/perf_event.rs exists
    Steps:
      1. cargo check 2>&1 | grep -q "error" && echo "FAIL" || echo "PASS: perf_event wrapper compiles"
    Expected Result: Compilation succeeds
    Evidence: .sisyphus/evidence/task-07-perf-event-compile.txt

  Scenario: Basic counter creation works
    Tool: Bash
    Preconditions: perf_event wrapper implemented
    Steps:
      1. cargo test --lib test_counter_creation 2>&1 | tee /tmp/counter-test.log
      2. grep -q "test result: ok" /tmp/counter-test.log && echo "PASS: Counter creation test passes"
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-07-counter-test.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add perf_event wrapper`
  - Files: src/core/perf_event.rs
  - Pre-commit: cargo check

- [x] 8. perf list implementation

  **What to do**:
  - Create src/commands/list.rs
  - Implement enumeration of available events
  - List hardware events: cpu-cycles, instructions, cache-references, cache-misses, branch-instructions, branch-misses
  - List software events: context-switches, cpu-migrations, page-faults, etc.
  - Use architecture abstraction from Task 5 for arch-specific events
  - Display event names with descriptions
  - Format output similar to standard perf list

  **Must NOT do**:
  - DO NOT implement detailed tracepoint enumeration (future work)
  - DO NOT hardcode events for all architectures (use traits)
  - DO NOT add filtering options unless needed

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires understanding of perf event types and architecture differences
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends only on Wave 1 tasks
  - **Parallel Group**: Wave 2 (with Tasks 9-14)
  - **Blocks**: Tasks F1-F4
  - **Blocked By**: Tasks 4, 5, 7

  **References**:
  **API References**:
  - perf-event2 events module: Hardware, Software enums

  **Pattern References**:
  - Standard perf list output format
  - Research findings show event enumeration should use /sys/bus/event_source/devices/

  **External References**:
  - perf list man page: https://man7.org/linux/man-pages/man1/perf-list.1.html

  **Acceptance Criteria**:
  - [ ] src/commands/list.rs created
  - [ ] cargo run -- list shows hardware and software events
  - [ ] Output format similar to standard perf list
  - [ ] Architecture-specific events listed correctly

  **QA Scenarios**:
  ```
  Scenario: perf list shows hardware events
    Tool: Bash
    Preconditions: list command implemented
    Steps:
      1. cargo run --release -- list 2>&1 | tee /tmp/perf-list.log
      2. grep -q "cpu-cycles" /tmp/perf-list.log && echo "PASS: cpu-cycles listed"
      3. grep -q "instructions" /tmp/perf-list.log && echo "PASS: instructions listed"
      4. grep -q "cache-misses" /tmp/perf-list.log && echo "PASS: cache-misses listed"
    Expected Result: Hardware events shown
    Evidence: .sisyphus/evidence/task-08-list-hardware.txt

  Scenario: perf list shows software events
    Tool: Bash
    Preconditions: list command implemented
    Steps:
      1. cargo run --release -- list 2>&1 | grep -q "context-switches" && echo "PASS: context-switches listed"
      2. cargo run --release -- list 2>&1 | grep -q "page-faults" && echo "PASS: page-faults listed"
    Expected Result: Software events shown
    Evidence: .sisyphus/evidence/task-08-list-software.txt

  Scenario: Compare with system perf (if available)
    Tool: Bash
    Preconditions: perf command available
    Steps:
      1. if command -v perf >/dev/null 2>&1; then
           cargo run --release -- list 2>/dev/null | sort > /tmp/perf-rs-list.txt
           perf list 2>/dev/null | sort > /tmp/perf-list.txt
           diff /tmp/perf-rs-list.txt /tmp/perf-list.txt | head -20
           echo "PASS: Comparison done"
         else
           echo "SKIP: perf not available"
         fi
    Expected Result: Output similar to system perf
    Evidence: .sisyphus/evidence/task-08-list-compare.txt
  ```

  **Commit**: YES
  - Message: `feat(commands): implement perf list`
  - Files: src/commands/list.rs, src/main.rs, src/commands/mod.rs
  - Pre-commit: cargo build --release

- [x] 9. perf stat - event counting

  **What to do**:
  - Create src/commands/stat.rs
  - Implement basic event counting for a command
  - Parse command to execute from CLI arguments
  - Create counter for specified events (default: cpu-cycles, instructions)
  - Execute command and measure events
  - Display results with event counts and percentages
  - Handle privilege failures gracefully

  **Must NOT do**:
  - DO NOT implement multi-event groups yet (Task 10)
  - DO NOT support -p PID mode yet (Task 10)
  - DO NOT add complex interval printing (future work)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Core functionality, requires process management and event measurement
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - independent of Task 8
  - **Parallel Group**: Wave 2 (with Tasks 8, 10-14)
  - **Blocks**: Task 10, Tasks F1-F4
  - **Blocked By**: Tasks 4, 6, 7

  **References**:
  **API References**:
  - perf-event2 Counter API: enable(), disable(), read()
  - nix crate for process execution

  **Pattern References**:
  - Standard perf stat output format
  - Research shows need for graceful privilege degradation

  **Acceptance Criteria**:
  - [ ] src/commands/stat.rs created
  - [ ] cargo run -- stat -- echo "test" shows event counts
  - [ ] Graceful handling of permission failures
  - [ ] Output format similar to standard perf stat

  **QA Scenarios**:
  ```
  Scenario: perf stat measures command execution
    Tool: Bash
    Preconditions: stat command implemented
    Steps:
      1. cargo run --release -- stat -- echo "test" 2>&1 | tee /tmp/perf-stat.log
      2. grep -q "cpu-cycles" /tmp/perf-stat.log && echo "PASS: cpu-cycles measured"
      3. grep -q "[0-9,]" /tmp/perf-stat.log && echo "PASS: Counts displayed"
    Expected Result: Event counts shown
    Evidence: .sisyphus/evidence/task-09-stat-basic.txt

  Scenario: perf stat handles permission errors
    Tool: Bash
    Preconditions: stat command implemented, restricted environment
    Steps:
      1. if [ $(cat /proc/sys/kernel/perf_event_paranoid) -gt 1 ]; then
           cargo run --release -- stat -- echo "test" 2>&1 | grep -q "permission\|paranoid\|error" && echo "PASS: Permission error handled"
         else
           echo "SKIP: Not in restricted mode"
         fi
    Expected Result: Graceful error message
    Evidence: .sisyphus/evidence/task-09-stat-permission.txt

  Scenario: perf stat compares with system perf
    Tool: Bash
    Preconditions: perf command available
    Steps:
      1. if command -v perf >/dev/null 2>&1; then
           cargo run --release -- stat -- sleep 0.1 2>&1 | head -5
           perf stat sleep 0.1 2>&1 | head -5
           echo "PASS: Comparison done"
         else
           echo "SKIP: perf not available"
         fi
    Expected Result: Similar output to system perf
    Evidence: .sisyphus/evidence/task-09-stat-compare.txt
  ```

  **Commit**: YES
  - Message: `feat(commands): implement perf stat - basic counting`
  - Files: src/commands/stat.rs
  - Pre-commit: cargo build --release

- [x] 10. perf stat - multi-event groups

  **What to do**:
  - Extend src/commands/stat.rs for multiple events
  - Support -e event1,event2,... option
  - Implement event groups using perf-event2 Group API
  - Support -p PID mode to attach to running process
  - Handle multiplexing when PMU counters are limited
  - Display all event counts with proper formatting

  **Must NOT do**:
  - DO NOT over-engineer multiplexing (use perf-event2's built-in support)
  - DO NOT add interval mode (future work)
  - DO NOT support per-CPU mode yet

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Extends Task 9 with more complex event management
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on Task 9 completion
  - **Parallel Group**: Wave 2 (with Tasks 8, 11-14)
  - **Blocks**: Tasks F1-F4
  - **Blocked By**: Task 9

  **References**:
  **API References**:
  - perf-event2 Group API for multiplexed measurement
  - perf-event2 Builder.pid() for process attachment

  **Acceptance Criteria**:
  - [ ] Multiple events can be specified with -e flag
  - [ ] cargo run -- stat -e cpu-cycles,instructions -- echo "test" works
  - [ ] Process attachment with -p PID works
  - [ ] Multiplexing handled correctly

  **QA Scenarios**:
  ```
  Scenario: Multiple events measured
    Tool: Bash
    Preconditions: stat command with -e support
    Steps:
      1. cargo run --release -- stat -e cpu-cycles,instructions -- echo "test" 2>&1 | tee /tmp/perf-stat-multi.log
      2. grep -q "cpu-cycles" /tmp/perf-stat-multi.log && echo "PASS: cpu-cycles in output"
      3. grep -q "instructions" /tmp/perf-stat-multi.log && echo "PASS: instructions in output"
    Expected Result: Both events measured
    Evidence: .sisyphus/evidence/task-10-stat-multi.txt

  Scenario: Process attachment works
    Tool: Bash
    Preconditions: stat command with -p support
    Steps:
      1. sleep 10 & PID=$!
      2. cargo run --release -- stat -p $PID sleep 0.5 2>&1 | tee /tmp/perf-stat-pid.log
      3. grep -q "cpu-cycles" /tmp/perf-stat-pid.log && echo "PASS: Process measured"
      4. kill $PID 2>/dev/null || true
    Expected Result: Process monitoring works
    Evidence: .sisyphus/evidence/task-10-stat-pid.txt
  ```

  **Commit**: YES
  - Message: `feat(commands): implement perf stat - multi-event groups`
  - Files: src/commands/stat.rs
  - Pre-commit: cargo build --release

- [x] 11. perf record - ring buffer setup

  **What to do**:
  - Create src/core/ringbuf.rs for ring buffer management
  - Implement memory-mapped ring buffer using perf-event2 Sampler
  - Handle buffer wraparound correctly
  - Implement buffer overflow detection
  - Support configurable buffer size
  - Add statistics for lost samples

  **Must NOT do**:
  - DO NOT implement custom ring buffer (use perf-event2's abstraction)
  - DO NOT skip overflow detection
  - DO NOT assume unlimited buffer space

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Complex memory management and kernel interface
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - independent of Tasks 8-10
  - **Parallel Group**: Wave 2 (with Tasks 8-10, 12-14)
  - **Blocks**: Tasks 12, 13
  - **Blocked By**: Tasks 6, 7

  **References**:
  **API References**:
  - perf-event2 Sampler API for ring buffer management
  - Memory-mapped I/O patterns

  **Pattern References**:
  - Research identified ring buffer management as critical challenge
  - Metis review noted overflow detection is essential

  **Acceptance Criteria**:
  - [ ] src/core/ringbuf.rs created
  - [ ] Ring buffer allocation and mmap working
  - [ ] Overflow detection implemented
  - [ ] Unit tests for buffer management pass

  **QA Scenarios**:
  ```
  Scenario: Ring buffer allocation works
    Tool: Bash
    Preconditions: ringbuf module implemented
    Steps:
      1. cargo test --lib ringbuf 2>&1 | tee /tmp/ringbuf-test.log
      2. grep -q "test result: ok" /tmp/ringbuf-test.log && echo "PASS: Ring buffer tests pass"
    Expected Result: Tests pass
    Evidence: .sisyphus/evidence/task-11-ringbuf-test.txt

  Scenario: Buffer overflow detected
    Tool: Bash
    Preconditions: ringbuf with overflow detection
    Steps:
      1. cargo test --lib test_buffer_overflow 2>&1 | grep -q "ok" && echo "PASS: Overflow detection works"
    Expected Result: Overflow detection functional
    Evidence: .sisyphus/evidence/task-11-overflow.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add ring buffer management`
  - Files: src/core/ringbuf.rs
  - Pre-commit: cargo test --lib ringbuf

- [x] 12. perf record - sampling and file writing

  **What to do**:
  - Create src/commands/record.rs
  - Implement sample-based profiling
  - Parse -e event, -F frequency, -o output options
  - Collect samples using ring buffer from Task 11
  - Write samples to perf.data file (use custom format for MVP)
  - Support call chain capture (frame pointers)
  - Handle graceful shutdown on signal (SIGINT, SIGTERM)

  **Must NOT do**:
  - DO NOT implement DWARF unwinding yet (frame pointers only)
  - DO NOT support Intel PT or ARM ETM (out of scope)
  - DO NOT skip signal handling for clean shutdown

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex integration of sampling, buffer management, and file I/O
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on Task 11
  - **Parallel Group**: Wave 2 (with Tasks 8-11, 13, 14)
  - **Blocks**: Tasks 13, 15, 18
  - **Blocked By**: Task 11

  **References**:
  **API References**:
  - perf-event2 Sampler for sample collection
  - Signal handling with nix crate

  **Pattern References**:
  - perf.data file format documentation
  - Research shows frame pointers are sufficient for MVP

  **External References**:
  - perf record man page: https://man7.org/linux/man-pages/man1/perf-record.1.html

  **Acceptance Criteria**:
  - [ ] src/commands/record.rs created
  - [ ] cargo run -- record -o /tmp/test.data -- sleep 0.1 creates file
  - [ ] perf.data file contains sample data
  - [ ] Signal handling for clean shutdown works

  **QA Scenarios**:
  ```
  Scenario: Basic recording creates file
    Tool: Bash
    Preconditions: record command implemented
    Steps:
      1. cargo run --release -- record -o /tmp/test-perf.data -- sleep 0.1 2>&1 | tee /tmp/record.log
      2. test -f /tmp/test-perf.data && echo "PASS: perf.data file created"
      3. test -s /tmp/test-perf.data && echo "PASS: perf.data file has content"
      4. rm -f /tmp/test-perf.data
    Expected Result: File created with data
    Evidence: .sisyphus/evidence/task-12-record-basic.txt

  Scenario: Recording with specific event
    Tool: Bash
    Preconditions: record command with -e support
    Steps:
      1. cargo run --release -- record -e cpu-cycles -o /tmp/test-perf2.data -- sleep 0.1 2>&1
      2. test -f /tmp/test-perf2.data && echo "PASS: Event-specific recording works"
      3. rm -f /tmp/test-perf2.data
    Expected Result: Event-specific recording works
    Evidence: .sisyphus/evidence/task-12-record-event.txt

  Scenario: Clean shutdown on signal
    Tool: Bash
    Preconditions: Signal handling implemented
    Steps:
      1. cargo run --release -- record -o /tmp/test-perf3.data -- sleep 10 & PID=$!
      2. sleep 0.5
      3. kill -SIGINT $PID 2>/dev/null
      4. wait $PID 2>/dev/null
      5. test -f /tmp/test-perf3.data && echo "PASS: Clean shutdown created file"
      6. rm -f /tmp/test-perf3.data
    Expected Result: File created even with signal
    Evidence: .sisyphus/evidence/task-12-record-signal.txt
  ```

  **Commit**: YES
  - Message: `feat(commands): implement perf record`
  - Files: src/commands/record.rs
  - Pre-commit: cargo build --release

- [x] 13. perf.data file format support

  **What to do**:
  - Create src/core/perf_data.rs
  - Implement perf.data file writing (header, events, metadata)
  - Implement perf.data file reading and parsing
  - Use linux-perf-data crate as foundation if possible
  - Handle version compatibility
  - Support basic event types: MMAP, COMM, FORK, EXIT, SAMPLE

  **Must NOT do**:
  - DO NOT implement all perf.data features (only what's needed for record/report)
  - DO NOT skip metadata (it's needed for symbol resolution)
  - DO NOT assume specific perf.data version

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Complex binary format parsing, may need custom implementation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - can work alongside Task 12
  - **Parallel Group**: Wave 2 (with Tasks 8-12, 14)
  - **Blocks**: Tasks 15, 18
  - **Blocked By**: Task 11

  **References**:
  **External References**:
  - perf.data format: https://github.com/torvalds/linux/blob/master/tools/perf/Documentation/perf.data-file-format.txt
  - linux-perf-data crate: https://crates.io/crates/linux-perf-data

  **Pattern References**:
  - Metis recommended using linux-perf-data crate initially

  **Acceptance Criteria**:
  - [ ] src/core/perf_data.rs created
  - [ ] File writing for basic events works
  - [ ] File reading and parsing works
  - [ ] Compatibility with standard perf.data files

  **QA Scenarios**:
  ```
  Scenario: perf.data writing works
    Tool: Bash
    Preconditions: perf_data module implemented
    Steps:
      1. cargo test --lib perf_data 2>&1 | tee /tmp/perf-data-test.log
      2. grep -q "test result: ok" /tmp/perf-data-test.log && echo "PASS: perf.data tests pass"
    Expected Result: Tests pass
    Evidence: .sisyphus/evidence/task-13-perf-data-test.txt

  Scenario: Files written by perf-rs can be read back
    Tool: Bash
    Preconditions: record and perf_data module work
    Steps:
      1. cargo run --release -- record -o /tmp/test-rs.data -- sleep 0.1
      2. cargo run --release -- report -i /tmp/test-rs.data 2>&1 | head -5
      3. echo "PASS: Round-trip works"
      4. rm -f /tmp/test-rs.data
    Expected Result: File can be read back
    Evidence: .sisyphus/evidence/task-13-perf-data-roundtrip.txt

  Scenario: Standard perf.data files can be read (if perf available)
    Tool: Bash
    Preconditions: perf command available, perf_data reading implemented
    Steps:
      1. if command -v perf >/dev/null 2>&1; then
           perf record -o /tmp/test-perf.data -- sleep 0.1 2>/dev/null
           cargo run --release -- report -i /tmp/test-perf.data 2>&1 | head -5
           echo "PASS: Standard perf.data readable"
           rm -f /tmp/test-perf.data
         else
           echo "SKIP: perf not available"
         fi
    Expected Result: Standard files readable
    Evidence: .sisyphus/evidence/task-13-perf-data-standard.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add perf.data file format support`
  - Files: src/core/perf_data.rs
  - Pre-commit: cargo test --lib perf_data

- [x] 14. Symbol resolution module

  **What to do**:
  - Create src/symbols/mod.rs with symbol resolution traits
  - Create src/symbols/elf.rs for ELF symbol parsing
  - Create src/symbols/kernel.rs for kernel symbols
  - Use gimli for DWARF debug info parsing
  - Implement address-to-symbol resolution
  - Support /proc/kallsyms for kernel symbols
  - Cache symbol tables for performance

  **Must NOT do**:
  - DO NOT implement JIT code support (future work)
  - DO NOT parse inline functions for MVP (basic DWARF only)
  - DO NOT skip symbol caching (performance critical)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex DWARF parsing, symbol table management, caching
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - independent of Tasks 8-13
  - **Parallel Group**: Wave 2 (with Tasks 8-13)
  - **Blocks**: Tasks 16, 17
  - **Blocked By**: Tasks 2, 3

  **References**:
  **API References**:
  - gimli crate for DWARF parsing
  - addr2line crate for higher-level symbol resolution
  - object crate for ELF reading

  **Pattern References**:
  - Metis recommended basic ELF + DWARF for MVP
  - /proc/kallsyms for kernel symbols

  **External References**:
  - gimli documentation: https://docs.rs/gimli/
  - addr2line documentation: https://docs.rs/addr2line/

  **Acceptance Criteria**:
  - [ ] src/symbols/ module created with traits
  - [ ] ELF symbol parsing works
  - [ ] DWARF debug info parsing works
  - [ ] Kernel symbol resolution from /proc/kallsyms works
  - [ ] Symbol caching implemented

  **QA Scenarios**:
  ```
  Scenario: ELF symbol parsing works
    Tool: Bash
    Preconditions: symbols module implemented
    Steps:
      1. cargo test --lib symbols 2>&1 | tee /tmp/symbols-test.log
      2. grep -q "test result: ok" /tmp/symbols-test.log && echo "PASS: Symbol tests pass"
    Expected Result: Tests pass
    Evidence: .sisyphus/evidence/task-14-symbols-test.txt

  Scenario: Kernel symbols can be read
    Tool: Bash
    Preconditions: kernel symbol resolution implemented
    Steps:
      1. cargo test --lib test_kernel_symbols 2>&1 | grep -q "ok" && echo "PASS: Kernel symbols readable"
    Expected Result: Test passes
    Evidence: .sisyphus/evidence/task-14-kernel-symbols.txt

  Scenario: Symbol resolution for test binary
    Tool: Bash
    Preconditions: Symbol resolution working
    Steps:
      1. cargo build --release
      2. cargo run --release -- record -o /tmp/test-sym.data -- ./target/release/perf-rs --help 2>/dev/null
      3. cargo run --release -- report -i /tmp/test-sym.data 2>&1 | grep -q "main\|perf" && echo "PASS: Symbols resolved"
      4. rm -f /tmp/test-sym.data
    Expected Result: Symbols shown in report
    Evidence: .sisyphus/evidence/task-14-symbols-resolve.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add symbol resolution`
  - Files: src/symbols/mod.rs, src/symbols/elf.rs, src/symbols/kernel.rs
  - Pre-commit: cargo test --lib symbols

- [x] 15. perf report - file parsing

  **What to do**:
  - Create src/commands/report.rs
  - Parse perf.data file using perf_data module from Task 13
  - Extract sample events with call chains
  - Build histogram of sample counts
  - Support -i input option
  - Handle missing or corrupted files gracefully

  **Must NOT do**:
  - DO NOT implement symbol resolution yet (Task 17)
  - DO NOT implement advanced analysis features (Task 16)
  - DO NOT assume file always exists or is valid

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Integration of file parsing and sample analysis
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on Tasks 12, 13
  - **Parallel Group**: Wave 3 (with Tasks 16-21)
  - **Blocks**: Tasks 16, 18
  - **Blocked By**: Tasks 4, 12, 13

  **References**:
  **API References**:
  - perf_data module for file reading
  - Histogram and aggregation patterns

  **Pattern References**:
  - Standard perf report output format
  - Research shows hotspot detection is primary goal

  **Acceptance Criteria**:
  - [ ] src/commands/report.rs created
  - [ ] cargo run -- report -i /tmp/test.data shows basic output
  - [ ] Sample histogram built correctly
  - [ ] Graceful handling of missing files

  **QA Scenarios**:
  ```
  Scenario: perf report parses file
    Tool: Bash
    Preconditions: report command implemented, test data exists
    Steps:
      1. cargo run --release -- record -o /tmp/test-report.data -- sleep 0.5
      2. cargo run --release -- report -i /tmp/test-report.data 2>&1 | tee /tmp/report.log
      3. grep -q "Overhead\|Samples\|Event" /tmp/report.log && echo "PASS: Report generates output"
      4. rm -f /tmp/test-report.data
    Expected Result: Report output shown
    Evidence: .sisyphus/evidence/task-15-report-parse.txt

  Scenario: Missing file handled gracefully
    Tool: Bash
    Preconditions: report command implemented
    Steps:
      1. cargo run --release -- report -i /tmp/nonexistent.data 2>&1 | tee /tmp/report-missing.log
      2. grep -q "error\|not found\|missing" /tmp/report-missing.log && echo "PASS: Missing file error handled"
    Expected Result: Error message shown
    Evidence: .sisyphus/evidence/task-15-report-missing.txt

  Scenario: Report shows sample counts
    Tool: Bash
    Preconditions: report with histogram working
    Steps:
      1. cargo run --release -- record -o /tmp/test-hist.data -- sleep 0.5
      2. cargo run --release -- report -i /tmp/test-hist.data 2>&1 | grep -E "[0-9]+\.[0-9]+%" && echo "PASS: Percentages shown"
      3. rm -f /tmp/test-hist.data
    Expected Result: Percentages displayed
    Evidence: .sisyphus/evidence/task-15-report-hist.txt
  ```

  **Commit**: YES
  - Message: `feat(commands): implement perf report - parsing`
  - Files: src/commands/report.rs
  - Pre-commit: cargo build --release

- [x] 16. perf report - hotspot analysis

  **What to do**:
  - Extend src/commands/report.rs with advanced analysis
  - Implement call chain aggregation
  - Sort functions by overhead percentage
  - Support --sort option for different sort criteria
  - Display call graphs (if call chains captured)
  - Show per-function sample counts and percentages

  **Must NOT do**:
  - DO NOT implement source annotation (out of scope)
  - DO NOT over-engineer sorting (basic sort by overhead)
  - DO NOT assume call chains always present

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex analysis with call chain processing and aggregation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on Tasks 14, 15
  - **Parallel Group**: Wave 3 (with Tasks 17-21)
  - **Blocks**: Task 17
  - **Blocked By**: Tasks 14, 15

  **References**:
  **Pattern References**:
  - Call chain aggregation algorithms
  - Standard perf report output format

  **Acceptance Criteria**:
  - [ ] Functions sorted by overhead
  - [ ] Call chain aggregation works
  - [ ] Top N hotspots displayed
  - [ ] --sort option functional

  **QA Scenarios**:
  ```
  Scenario: Functions sorted by overhead
    Tool: Bash
    Preconditions: report with sorting implemented
    Steps:
      1. cargo run --release -- record -o /tmp/test-sort.data -- sleep 0.5
      2. cargo run --release -- report -i /tmp/test-sort.data 2>&1 | head -10 | tee /tmp/report-sorted.log
      3. grep -q "%" /tmp/report-sorted.log && echo "PASS: Percentages present"
      4. rm -f /tmp/test-sort.data
    Expected Result: Sorted output shown
    Evidence: .sisyphus/evidence/task-16-report-sort.txt

  Scenario: Call chains aggregated
    Tool: Bash
    Preconditions: report with call chain support
    Steps:
      1. cargo run --release -- record -g -o /tmp/test-callchain.data -- sleep 0.5
      2. cargo run --release -- report -i /tmp/test-callchain.data 2>&1 | grep -q "callchain\|stack" && echo "PASS: Call chains shown" || echo "INFO: Call chains may not be available"
      3. rm -f /tmp/test-callchain.data
    Expected Result: Call chain information shown
    Evidence: .sisyphus/evidence/task-16-report-callchain.txt
  ```

  **Commit**: YES
  - Message: `feat(commands): implement perf report - analysis`
  - Files: src/commands/report.rs
  - Pre-commit: cargo build --release

- [x] 17. perf report - symbol integration

  **What to do**:
  - Integrate symbol resolution from Task 14 into report
  - Translate addresses to function names
  - Show source file and line numbers (if available in DWARF)
  - Handle missing symbols gracefully (show hex address)
  - Display demangled Rust symbols

  **Must NOT do**:
  - DO NOT fail if symbols missing (show address instead)
  - DO NOT implement inline function expansion (future work)
  - DO NOT skip symbol resolution for kernel addresses

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Integration task combining symbols and report
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on Tasks 14, 15, 16
  - **Parallel Group**: Wave 3 (with Tasks 18-21)
  - **Blocks**: Tasks F1-F4
  - **Blocked By**: Tasks 14, 15, 16

  **References**:
  **API References**:
  - symbols module from Task 14
  - report module from Task 15

  **Pattern References**:
  - Research shows symbol resolution is critical for usability
  - Metis recommended graceful handling of missing symbols

  **Acceptance Criteria**:
  - [ ] Function names displayed instead of hex addresses
  - [ ] Source file/line shown when available
  - [ ] Kernel symbols resolved from kallsyms
  - [ ] Missing symbols handled gracefully

  **QA Scenarios**:
  ```
  Scenario: Symbols shown in report
    Tool: Bash
    Preconditions: Symbol integration complete
    Steps:
      1. cargo build --release
      2. cargo run --release -- record -o /tmp/test-symbols.data -- ./target/release/perf-rs --help 2>/dev/null
      3. cargo run --release -- report -i /tmp/test-symbols.data 2>&1 | tee /tmp/report-symbols.log
      4. grep -q "main\|perf\|run" /tmp/report-symbols.log && echo "PASS: Function names shown"
      5. rm -f /tmp/test-symbols.data
    Expected Result: Function names displayed
    Evidence: .sisyphus/evidence/task-17-report-symbols.txt

  Scenario: Missing symbols show address
    Tool: Bash
    Preconditions: Symbol integration with fallback
    Steps:
      1. cargo run --release -- record -o /tmp/test-nosym.data -- /bin/ls 2>/dev/null
      2. cargo run --release -- report -i /tmp/test-nosym.data 2>&1 | grep -E "0x[0-9a-f]+" && echo "PASS: Addresses shown for missing symbols"
      3. rm -f /tmp/test-nosym.data
    Expected Result: Hex addresses shown
    Evidence: .sisyphus/evidence/task-17-report-nosym.txt
  ```

  **Commit**: YES
  - Message: `feat(commands): integrate symbols into perf report`
  - Files: src/commands/report.rs
  - Pre-commit: cargo build --release

- [x] 18. perf script implementation

  **What to do**:
  - Create src/commands/script.rs
  - Parse perf.data file
  - Dump each sample event in human-readable format
  - Show timestamp, PID, TID, CPU, event name, address
  - Integrate symbol resolution for addresses
  - Support basic scripting output format

  **Must NOT do**:
  - DO NOT implement Python scripting support (future work)
  - DO NOT over-engineer output format (standard perf script format)
  - DO NOT skip symbol resolution

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Integration task similar to report, but different output format
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on Tasks 12, 13, 15
  - **Parallel Group**: Wave 3 (with Tasks 19-21)
  - **Blocks**: Tasks F1-F4
  - **Blocked By**: Tasks 4, 12, 13, 15

  **References**:
  **Pattern References**:
  - Standard perf script output format

  **External References**:
  - perf script man page: https://man7.org/linux/man-pages/man1/perf-script.1.html

  **Acceptance Criteria**:
  - [ ] src/commands/script.rs created
  - [ ] cargo run -- script -i /tmp/test.data shows sample trace
  - [ ] Timestamps and PIDs displayed
  - [ ] Symbols resolved and shown

  **QA Scenarios**:
  ```
  Scenario: perf script dumps trace data
    Tool: Bash
    Preconditions: script command implemented
    Steps:
      1. cargo run --release -- record -o /tmp/test-script.data -- sleep 0.5
      2. cargo run --release -- script -i /tmp/test-script.data 2>&1 | tee /tmp/script.log | head -20
      3. grep -E "[0-9]+\.[0-9]+" /tmp/script.log && echo "PASS: Timestamps present"
      4. rm -f /tmp/test-script.data
    Expected Result: Trace data shown
    Evidence: .sisyphus/evidence/task-18-script-trace.txt

  Scenario: Script shows symbols
    Tool: Bash
    Preconditions: script with symbol resolution
    Steps:
      1. cargo build --release
      2. cargo run --release -- record -o /tmp/test-sym-script.data -- ./target/release/perf-rs --help 2>/dev/null
      3. cargo run --release -- script -i /tmp/test-sym-script.data 2>&1 | head -10 | grep -q "[a-zA-Z_]" && echo "PASS: Symbols shown"
      4. rm -f /tmp/test-sym-script.data
    Expected Result: Symbols in script output
    Evidence: .sisyphus/evidence/task-18-script-symbols.txt
  ```

  **Commit**: YES
  - Message: `feat(commands): implement perf script`
  - Files: src/commands/script.rs
  - Pre-commit: cargo build --release

- [x] 19. Multi-architecture event discovery

  **What to do**:
  - Implement architecture-specific PMU event enumeration
  - Create src/arch/x86_64.rs with Intel/AMD events
  - Create src/arch/arm64.rs with ARM PMU events
  - Create src/arch/riscv64.rs with RISC-V PMU events
  - Parse /sys/bus/event_source/devices/ for runtime discovery
  - Implement arch detection at runtime (use cfg and runtime checks)
  - Provide fallback to generic events for unsupported architectures

  **Must NOT do**:
  - DO NOT assume events are identical across architectures
  - DO NOT hardcode all events (use sysfs when possible)
  - DO NOT skip architecture detection

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires architecture-specific knowledge and sysfs parsing
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on Tasks 5, 7
  - **Parallel Group**: Wave 3 (with Tasks 20, 21)
  - **Blocks**: Tasks F1-F4
  - **Blocked By**: Tasks 5, 7

  **References**:
  **Pattern References**:
  - Metis recommended parsing /sys/bus/event_source/devices/
  - Research shows architecture-specific challenges need abstraction

  **External References**:
  - Intel PMU documentation
  - ARM PMU architecture manual
  - RISC-V PMU specification

  **Acceptance Criteria**:
  - [ ] Architecture-specific modules implemented
  - [ ] Runtime architecture detection works
  - [ ] Events enumerated correctly per architecture
  - [ ] Fallback to generic events when needed

  **QA Scenarios**:
  ```
  Scenario: Architecture detected correctly
    Tool: Bash
    Preconditions: arch detection implemented
    Steps:
      1. ARCH=$(uname -m)
      2. cargo run --release -- list 2>&1 | grep -q "$ARCH\|cpu-cycles" && echo "PASS: Architecture $ARCH supported"
    Expected Result: Architecture recognized
    Evidence: .sisyphus/evidence/task-19-arch-detect.txt

  Scenario: Architecture-specific events listed
    Tool: Bash
    Preconditions: arch-specific events implemented
    Steps:
      1. ARCH=$(uname -m)
      2. case $ARCH in
           x86_64)
             cargo run --release -- list | grep -q "branch-instructions" && echo "PASS: x86_64 events listed"
             ;;
           aarch64)
             cargo run --release -- list | grep -q "cpu-cycles" && echo "PASS: arm64 events listed"
             ;;
           riscv64)
             cargo run --release -- list | grep -q "cpu-cycles" && echo "PASS: riscv64 events listed"
             ;;
           *)
             echo "INFO: Architecture $ARCH may have limited support"
             ;;
         esac
    Expected Result: Architecture-specific events shown
    Evidence: .sisyphus/evidence/task-19-arch-events.txt

  Scenario: Sysfs event discovery works
    Tool: Bash
    Preconditions: sysfs parsing implemented
    Steps:
      1. test -d /sys/bus/event_source/devices/cpu && echo "PASS: CPU PMU available"
      2. cargo run --release -- list 2>&1 | wc -l
    Expected Result: Events discovered from sysfs
    Evidence: .sisyphus/evidence/task-19-sysfs-discovery.txt
  ```

  **Commit**: YES
  - Message: `feat(arch): implement multi-architecture event discovery`
  - Files: src/arch/x86_64.rs, src/arch/arm64.rs, src/arch/riscv64.rs, src/arch/mod.rs
  - Pre-commit: cargo build --release

- [x] 20. Error handling and diagnostics

  **What to do**:
  - Improve error messages throughout the codebase
  - Add context to all errors using anyhow's .context()
  - Implement helpful diagnostics for common failure modes
  - Add --verbose flag for detailed debugging output
  - Provide actionable suggestions for permission errors
  - Add logging support (env_logger or tracing)

  **Must NOT do**:
  - DO NOT use unwrap() in production code paths
  - DO NOT hide error causes with overly generic messages
  - DO NOT skip error context

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires understanding of all error paths and user experience
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on Tasks 3, 6
  - **Parallel Group**: Wave 3 (with Task 21)
  - **Blocks**: Tasks F1-F4
  - **Blocked By**: Tasks 3, 6

  **References**:
  **Pattern References**:
  - Metis emphasized comprehensive error handling
  - anyhow for application errors, thiserror for library errors

  **Acceptance Criteria**:
  - [ ] All error paths have context
  - [ ] Permission errors show actionable suggestions
  - [ ] --verbose flag provides debugging info
  - [ ] No unwrap() in production code

  **QA Scenarios**:
  ```
  Scenario: Permission errors are helpful
    Tool: Bash
    Preconditions: Improved error messages
    Steps:
      1. if [ $(cat /proc/sys/kernel/perf_event_paranoid) -gt 1 ]; then
           cargo run --release -- stat -- echo "test" 2>&1 | tee /tmp/perm-error.log
           grep -q "permission\|sudo\|paranoid\|CAP_" /tmp/perm-error.log && echo "PASS: Helpful permission error"
         else
           echo "SKIP: Not in restricted mode"
         fi
    Expected Result: Actionable error message
    Evidence: .sisyphus/evidence/task-20-error-permission.txt

  Scenario: Verbose mode provides details
    Tool: Bash
    Preconditions: --verbose flag implemented
    Steps:
      1. cargo run --release -- --verbose stat -- echo "test" 2>&1 | tee /tmp/verbose.log
      2. wc -l /tmp/verbose.log | awk '{if ($1 > 5) print "PASS: Verbose output detailed"; else print "FAIL: Not enough detail"}'
    Expected Result: More output in verbose mode
    Evidence: .sisyphus/evidence/task-20-verbose.txt

  Scenario: No unwrap in production code
    Tool: Bash
    Preconditions: Code review complete
    Steps:
      1. grep -r "\.unwrap()" src/ --exclude-dir=target | grep -v "// test" | grep -v "#\[test\]" > /tmp/unwrap-check.txt
      2. if [ -s /tmp/unwrap-check.txt ]; then
           echo "FAIL: Found unwrap() calls:"
           cat /tmp/unwrap-check.txt
         else
           echo "PASS: No unwrap() in production code"
         fi
    Expected Result: No unwrap() calls found
    Evidence: .sisyphus/evidence/task-20-no-unwrap.txt
  ```

  **Commit**: YES
  - Message: `feat: improve error handling and diagnostics`
  - Files: src/error.rs, src/main.rs, src/commands/*.rs
  - Pre-commit: cargo clippy -- -D warnings

- [ ] 21. Documentation and examples

  **What to do**:
  - Create comprehensive README.md with:
    - Project description and features
    - Installation instructions
    - Usage examples for each subcommand
    - Architecture support details
    - Requirements (kernel version, permissions)
    - Comparison with standard perf tool
    - Limitations and known issues
  - Add inline documentation for public APIs
  - Create examples/ directory with example commands
  - Document privilege requirements clearly

  **Must NOT do**:
  - DO NOT add excessive documentation (keep it practical)
  - DO NOT skip usage examples
  - DO NOT forget to mention limitations

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: Documentation writing task
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES - depends on all implementation tasks
  - **Parallel Group**: Wave 3 (last task)
  - **Blocks**: Tasks F1-F4
  - **Blocked By**: Tasks 8-20

  **References**:
  **Pattern References**:
  - ripgrep README as example of good Rust project documentation
  - Standard perf tool documentation

  **Acceptance Criteria**:
  - [ ] README.md created with all sections
  - [ ] Usage examples for each subcommand
  - [ ] Architecture support documented
  - [ ] Examples directory created

  **QA Scenarios**:
  ```
  Scenario: README exists and is comprehensive
    Tool: Bash
    Preconditions: README.md created
    Steps:
      1. test -f README.md && echo "PASS: README exists"
      2. grep -q "perf-rs" README.md && echo "PASS: Project name mentioned"
      3. grep -q "INSTALL\|Usage" README.md && echo "PASS: Installation/usage sections present"
    Expected Result: README complete
    Evidence: .sisyphus/evidence/task-21-readme.txt

  Scenario: Usage examples are valid
    Tool: Bash
    Preconditions: Examples documented
    Steps:
      1. grep -A5 "perf list" README.md | grep "cargo run" | head -1 | bash 2>&1 | tee /tmp/example-list.log
      2. grep -q "cpu-cycles\|instructions" /tmp/example-list.log && echo "PASS: Example works"
    Expected Result: Examples execute successfully
    Evidence: .sisyphus/evidence/task-21-examples.txt
  ```

  **Commit**: YES
  - Message: `docs: add documentation and examples`
  - Files: README.md, examples/
  - Pre-commit: None

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Rejection → fix → re-run.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy -- -D warnings` + `cargo test` + `cargo build --release`. Review all changed files for: `as any`/`@ts-ignore`, empty catches, unwrap() in production code, commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names (data/result/item/temp).
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Integration Testing** — `deep`
  Test all subcommands against real workloads. Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-command integration (record → report → script). Test edge cases: permission failures, missing files, large datasets. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination: Task N touching Task M's files. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **Task 1**: `chore: initialize git repository with .gitignore` — .gitignore
- **Task 2**: `chore: add Cargo.toml with dependencies` — Cargo.toml, Cargo.lock
- **Task 3**: `feat(core): add error types` — src/error.rs
- **Task 4**: `feat(cli): add CLI structure with clap` — src/cli.rs, src/main.rs
- **Task 5**: `feat(arch): add architecture abstraction traits` — src/arch/mod.rs
- **Task 6**: `feat(core): add privilege checking` — src/core/privilege.rs
- **Task 7**: `feat(core): add perf_event wrapper` — src/core/perf_event.rs
- **Task 8**: `feat(commands): implement perf list` — src/commands/list.rs, src/main.rs
- **Task 9**: `feat(commands): implement perf stat - basic counting` — src/commands/stat.rs
- **Task 10**: `feat(commands): implement perf stat - multi-event groups` — src/commands/stat.rs
- **Task 11**: `feat(core): add ring buffer management` — src/core/ringbuf.rs
- **Task 12**: `feat(commands): implement perf record` — src/commands/record.rs
- **Task 13**: `feat(core): add perf.data file format support` — src/core/perf_data.rs
- **Task 14**: `feat(core): add symbol resolution` — src/symbols/mod.rs
- **Task 15**: `feat(commands): implement perf report - parsing` — src/commands/report.rs
- **Task 16**: `feat(commands): implement perf report - analysis` — src/commands/report.rs
- **Task 17**: `feat(commands): integrate symbols into perf report` — src/commands/report.rs
- **Task 18**: `feat(commands): implement perf script` — src/commands/script.rs
- **Task 19**: `feat(arch): implement multi-architecture event discovery` — src/arch/x86_64.rs, src/arch/arm64.rs, src/arch/riscv64.rs
- **Task 20**: `feat: improve error handling and diagnostics` — src/error.rs, src/main.rs
- **Task 21**: `docs: add documentation and examples` — README.md, examples/

---

## Success Criteria

### Verification Commands
```bash
# Verify all subcommands work
cargo run --release -- list | head -10
cargo run --release -- stat -- echo "test"
cargo run --release -- record -o /tmp/test.data -- sleep 0.1
cargo run --release -- report -i /tmp/test.data
cargo run --release -- script -i /tmp/test.data

# Verify architecture support
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
cargo build --release --target riscv64gc-unknown-linux-gnu

# Verify quality
cargo clippy -- -D warnings
cargo test
cargo build --release
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Git repository properly initialized
- [ ] All tasks committed
- [ ] Multi-architecture support working
- [ ] Privilege checking functional
- [ ] Error handling comprehensive
