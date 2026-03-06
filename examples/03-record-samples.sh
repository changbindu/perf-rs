#!/bin/bash
# Example: Record performance samples with perf-rs record
#
# This example demonstrates how to record performance samples
# for profiling applications.

set -e

echo "=== perf-rs record examples ==="
echo ""

# Basic record with frequency
echo "1. Record at 99 Hz frequency (common for profiling):"
echo "   sudo perf-rs record --frequency 99 -- ./your_program"
echo ""

# Record specific event
echo "2. Record CPU cycles at 999 Hz:"
echo "   sudo perf-rs record --event cpu-cycles --frequency 999 -- ./your_program"
echo ""

# Use sampling period instead of frequency
echo "3. Use sampling period (record every 100,000 instructions):"
echo "   sudo perf-rs record --event instructions --period 100000 -- ./your_program"
echo ""

# Monitor running process
echo "4. Record from a running process (PID 1234):"
echo "   sudo perf-rs record --pid 1234 --frequency 99"
echo ""

# Specify output file
echo "5. Save to custom output file:"
echo "   sudo perf-rs record --output my_profile.data --frequency 99 -- ./your_program"
echo ""

# Record for profiling cache behavior
echo "6. Profile cache misses:"
echo "   sudo perf-rs record --event cache-misses --frequency 99 -- ./your_program"
echo ""

# Record for branch prediction analysis
echo "7. Profile branch prediction:"
echo "   sudo perf-rs record --event branch-misses --frequency 999 -- ./your_program"
echo ""

echo "Tips for sampling frequency:"
echo "  99 Hz    - Low overhead, good for long-running processes"
echo "  999 Hz   - Medium overhead, good balance"
echo "  9999 Hz  - High overhead, detailed short-lived events"
echo ""
echo "Note: Output defaults to 'perf.data' if not specified."