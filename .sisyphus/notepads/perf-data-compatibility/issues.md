# Issues - perf-data-compatibility

## 2026-03-07 - Task 1: Research perf.data Format

### Issues Found

1. **Current format is incompatible**: perf-rs uses custom "PERFRS01" magic, not Linux perf "PERFILE2"
2. **Event type numbers wrong**: Sample=3 should be 9, Comm=2 should be 3
3. **Missing structures**: No perf_event_attr array, no feature bits section
4. **Header size wrong**: 64 bytes vs required 104 bytes
5. **No alignment support**: Current code doesn't enforce 8-byte alignment

### Blocking Issues

None at this time.

### Potential Issues

1. **string padding**: Strings must be padded to 64-byte alignment - need to implement
2. **feature bits**: 256-bit bitmap for optional features - may be complex
3. **sample_id_all**: If set, all events need sample_id structure at end
4. **kernel version differences**: perf_event_attr size varies (64-144 bytes)

### Resolution Status

- [x] Researched format specification
- [x] Created reference files
- [ ] Implemented writer core
- [ ] Implemented event writers
- [ ] Integrated with record command
- [ ] Created validation tests