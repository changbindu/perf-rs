# Decisions - perf-data-compatibility

## 2026-03-07 - Task 1: Research perf.data Format

### Architecture Decisions

1. **Complete rewrite of perf_data.rs**: Custom format is too different from Linux perf
2. **Implement from scratch**: Not using `linux-perf-data` crate yet - need to verify if it supports writing
3. **Use byteorder crate**: Explicit little-endian serialization for reliability
4. **Target kernel 6.17**: Use perf_event_attr size of 136 bytes
5. **Minimal features for v1**: Implement core events (SAMPLE, MMAP, COMM) and FINISHED_ROUND only

### Format Decisions

1. **Header structure**: Follow Linux perf exactly (104 bytes)
2. **Magic number**: PERFILE2 (0x50455246494c4532)
3. **Event types**: Use Linux perf constants
4. **Alignment**: 8-byte alignment for all records
5. **Byte order**: Little-endian explicitly

### Implementation Phases

1. Phase 1: Writer core (header, attributes, FINISHED_ROUND)
2. Phase 2: Event writers (SAMPLE, MMAP, COMM)
3. Phase 3: Integration with record command
4. Phase 4: Validation tests

### Future Considerations

- May add `linux-perf-data` crate for reading if needed
- Feature bits can be added incrementally
- Support for older kernel versions if needed