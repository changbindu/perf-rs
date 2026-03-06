#!/bin/bash
# Example: Complete profiling workflow
#
# This example demonstrates a complete profiling workflow
# from recording to analysis.

set -e

PROGRAM="${1:-./target/release/your_program}"
DATA_FILE="profile_$(date +%Y%m%d_%H%M%S).data"

echo "=== Complete profiling workflow ==="
echo ""
echo "Target program: $PROGRAM"
echo "Output file: $DATA_FILE"
echo ""

# Step 1: List available events
echo "Step 1: Checking available events..."
echo "Command: sudo perf-rs list --filter cache"
# sudo perf-rs list --filter cache
echo ""

# Step 2: Build the program (if needed)
echo "Step 2: Building program..."
echo "Command: cargo build --release"
# cargo build --release
echo ""

# Step 3: Record performance samples
echo "Step 3: Recording samples..."
echo "Command: sudo perf-rs record --output $DATA_FILE --frequency 999 -- $PROGRAM"
# sudo perf-rs record --output "$DATA_FILE" --frequency 999 -- "$PROGRAM"
echo ""

# Step 4: Analyze the data
echo "Step 4: Generating report..."
echo "Command: sudo perf-rs report --input $DATA_FILE --top 10"
# sudo perf-rs report --input "$DATA_FILE" --top 10
echo ""

# Step 5: Export trace data
echo "Step 5: Exporting trace data..."
echo "Command: sudo perf-rs script --input $DATA_FILE --callchain > ${DATA_FILE%.data}.trace"
# sudo perf-rs script --input "$DATA_FILE" --callchain > "${DATA_FILE%.data}.trace"
echo ""

# Optional: Export as JSON for further analysis
echo "Optional: Export as JSON..."
echo "Command: sudo perf-rs report --input $DATA_FILE --format json > ${DATA_FILE%.data}_report.json"
# sudo perf-rs report --input "$DATA_FILE" --format json > "${DATA_FILE%.data}_report.json"
echo ""

echo "Profiling complete!"
echo "Generated files:"
echo "  - $DATA_FILE (raw performance data)"
echo "  - ${DATA_FILE%.data}.trace (text trace)"
echo "  - ${DATA_FILE%.data}_report.json (JSON report)"
echo ""
echo "Tip: Add debug symbols to your binary for better symbol resolution:"
echo "  Keep debug info: cargo build --release (no --strip)"
echo "  Or use: cargo build (debug mode has symbols by default)"