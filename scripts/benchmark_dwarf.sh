#!/bin/bash
#
# Benchmark script comparing FP vs DWARF unwinding overhead
# Measures: time to record 5 seconds at 99 Hz, and perf.data file size
#
# NOTE: This script requires root privileges or CAP_PERFMON capability
# to run perf-rs record commands.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EVIDENCE_DIR="$PROJECT_ROOT/.sisyphus/evidence"
EVIDENCE_FILE="$EVIDENCE_DIR/task-13-perf.txt"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Benchmark parameters
FREQUENCY=99
DURATION=10
OUTPUT_FP="/tmp/perf-fp.data"
OUTPUT_DWARF="/tmp/perf-dwarf.data"
WORKLOAD_SCRIPT="$SCRIPT_DIR/workload.sh"

echo -e "${BLUE}=== DWARF vs FP Unwinding Benchmark ===${NC}"
echo ""

# Step 1: Build the project in release mode
echo -e "${YELLOW}Step 1: Building project in release mode...${NC}"
cd "$PROJECT_ROOT"
cargo build --release 2>&1 | tail -5
PERF_RS="$PROJECT_ROOT/target/release/perf-rs"

if [ ! -f "$PERF_RS" ]; then
    echo -e "${RED}ERROR: perf-rs binary not found at $PERF_RS${NC}"
    exit 1
fi
echo -e "${GREEN}Build complete.${NC}"
echo ""

# Step 2: Check privileges
echo -e "${YELLOW}Step 2: Checking privileges...${NC}"
if [ "$(id -u)" -ne 0 ]; then
    echo -e "${YELLOW}WARNING: Not running as root. perf-rs record may fail.${NC}"
    echo -e "${YELLOW}         Consider running with: sudo $0${NC}"
    echo ""
fi

# Step 3: Clean up any previous runs
rm -f "$OUTPUT_FP" "$OUTPUT_DWARF"

# Step 4: Run FP benchmark
echo -e "${YELLOW}Step 3: Running FP unwinding benchmark (${DURATION}s at ${FREQUENCY}Hz)...${NC}"
echo "Command: $PERF_RS record --call-graph=fp --frequency $FREQUENCY --output $OUTPUT_FP -- $WORKLOAD_SCRIPT"
echo ""

START_FP=$(date +%s.%N)
$PERF_RS record --call-graph=fp --frequency $FREQUENCY --output "$OUTPUT_FP" -- "$WORKLOAD_SCRIPT" 2>&1 || {
    echo -e "${RED}ERROR: FP benchmark failed. Are you running as root?${NC}"
    exit 1
}
END_FP=$(date +%s.%N)
TIME_FP=$(echo "$END_FP - $START_FP" | bc)

if [ -f "$OUTPUT_FP" ]; then
    SIZE_FP=$(stat -c%s "$OUTPUT_FP")
    echo -e "${GREEN}FP benchmark complete.${NC}"
    echo "  Time: ${TIME_FP}s"
    echo "  File size: $SIZE_FP bytes"
else
    echo -e "${RED}ERROR: FP output file not created${NC}"
    exit 1
fi
echo ""

# Step 5: Run DWARF benchmark
echo -e "${YELLOW}Step 4: Running DWARF unwinding benchmark (${DURATION}s at ${FREQUENCY}Hz)...${NC}"
echo "Command: $PERF_RS record --call-graph=dwarf --frequency $FREQUENCY --output $OUTPUT_DWARF -- $WORKLOAD_SCRIPT"
echo ""

START_DWARF=$(date +%s.%N)
$PERF_RS record --call-graph=dwarf --frequency $FREQUENCY --output "$OUTPUT_DWARF" -- "$WORKLOAD_SCRIPT" 2>&1 || {
    echo -e "${RED}ERROR: DWARF benchmark failed. Are you running as root?${NC}"
    exit 1
}
END_DWARF=$(date +%s.%N)
TIME_DWARF=$(echo "$END_DWARF - $START_DWARF" | bc)

if [ -f "$OUTPUT_DWARF" ]; then
    SIZE_DWARF=$(stat -c%s "$OUTPUT_DWARF")
    echo -e "${GREEN}DWARF benchmark complete.${NC}"
    echo "  Time: ${TIME_DWARF}s"
    echo "  File size: $SIZE_DWARF bytes"
else
    echo -e "${RED}ERROR: DWARF output file not created${NC}"
    exit 1
fi
echo ""

# Step 6: Calculate overhead
echo -e "${BLUE}=== Results ===${NC}"
echo ""

# Time overhead
TIME_OVERHEAD=$(echo "scale=2; (($TIME_DWARF - $TIME_FP) / $TIME_FP) * 100" | bc)
echo "Time Comparison:"
echo "  FP:    ${TIME_FP}s"
echo "  DWARF: ${TIME_DWARF}s"
echo "  Overhead: ${TIME_OVERHEAD}%"
echo ""

# File size overhead
SIZE_OVERHEAD=$(echo "scale=2; (($SIZE_DWARF - $SIZE_FP) / $SIZE_FP) * 100" | bc)
echo "File Size Comparison:"
echo "  FP:    $SIZE_FP bytes ($(echo "scale=2; $SIZE_FP / 1024" | bc) KB)"
echo "  DWARF: $SIZE_DWARF bytes ($(echo "scale=2; $SIZE_DWARF / 1024" | bc) KB)"
echo "  Overhead: ${SIZE_OVERHEAD}%"
echo ""

# Step 7: Determine pass/fail
TARGET_OVERHEAD=10
PASS=true

# Check if overhead exceeds target
if (( $(echo "$TIME_OVERHEAD > $TARGET_OVERHEAD" | bc -l) )); then
    echo -e "${RED}FAIL: Time overhead (${TIME_OVERHEAD}%) exceeds target (${TARGET_OVERHEAD}%)${NC}"
    PASS=false
else
    echo -e "${GREEN}PASS: Time overhead (${TIME_OVERHEAD}%) within target (${TARGET_OVERHEAD}%)${NC}"
fi

if (( $(echo "$SIZE_OVERHEAD > $TARGET_OVERHEAD" | bc -l) )); then
    echo -e "${YELLOW}NOTE: File size overhead (${SIZE_OVERHEAD}%) exceeds target (${TARGET_OVERHEAD}%)${NC}"
    echo -e "${YELLOW}     This is expected as DWARF captures more stack data.${NC}"
else
    echo -e "${GREEN}PASS: File size overhead (${SIZE_OVERHEAD}%) within target (${TARGET_OVERHEAD}%)${NC}"
fi
echo ""

# Step 8: Document results
echo -e "${YELLOW}Step 5: Documenting results...${NC}"
mkdir -p "$EVIDENCE_DIR"

cat > "$EVIDENCE_FILE" << EOF
# DWARF vs FP Unwinding Benchmark Results

Date: $(date)
Platform: $(uname -a)
Rust Version: $(rustc --version)

## Benchmark Parameters
- Frequency: ${FREQUENCY} Hz
- Duration: ${DURATION} seconds
- Workload: CPU-intensive Perl loop

## Results

### Time Comparison
| Method | Time (s) |
|--------|----------|
| FP     | ${TIME_FP} |
| DWARF  | ${TIME_DWARF} |
| Overhead | ${TIME_OVERHEAD}% |

### File Size Comparison
| Method | Size (bytes) | Size (KB) |
|--------|--------------|-----------|
| FP     | ${SIZE_FP} | $(echo "scale=2; $SIZE_FP / 1024" | bc) |
| DWARF  | ${SIZE_DWARF} | $(echo "scale=2; $SIZE_DWARF / 1024" | bc) |
| Overhead | ${SIZE_OVERHEAD}% |

## Target
- Time overhead: < ${TARGET_OVERHEAD}%

## Status
EOF

if [ "$PASS" = true ]; then
    echo "- [x] Overhead < ${TARGET_OVERHEAD}% confirmed" >> "$EVIDENCE_FILE"
else
    echo "- [ ] Overhead < ${TARGET_OVERHEAD}% NOT confirmed (actual: ${TIME_OVERHEAD}%)" >> "$EVIDENCE_FILE"
fi

echo ""
echo -e "${GREEN}Results documented in: $EVIDENCE_FILE${NC}"

# Cleanup
rm -f "$OUTPUT_FP" "$OUTPUT_DWARF"

# Exit with appropriate code
if [ "$PASS" = true ]; then
    echo -e "${GREEN}Benchmark PASSED${NC}"
    exit 0
else
    echo -e "${RED}Benchmark FAILED${NC}"
    exit 1
fi