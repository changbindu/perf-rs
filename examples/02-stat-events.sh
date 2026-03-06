#!/bin/bash
# Example: Count performance events with perf-rs stat
#
# This example demonstrates how to count performance events
# during command execution or for running processes.

set -e

echo "=== perf-rs stat examples ==="
echo ""

# Basic stat - count default events
echo "1. Count events for a simple command:"
echo "   sudo perf-rs stat -- ls -la"
echo ""

# Count specific events
echo "2. Count CPU cycles and instructions:"
echo "   sudo perf-rs stat --event cpu-cycles,instructions -- ./your_program"
echo ""

# Monitor a running process
echo "3. Monitor a running process (PID 1234):"
echo "   sudo perf-rs stat --pid 1234 --event cache-misses"
echo ""

# Count multiple hardware events
echo "4. Count multiple hardware events:"
echo "   sudo perf-rs stat --event cpu-cycles,instructions,cache-references,cache-misses,branch-instructions,branch-misses -- ./benchmark"
echo ""

# Use event aliases
echo "5. Use event aliases (cycles instead of cpu-cycles):"
echo "   sudo perf-rs stat --event cycles,instructions -- ./your_program"
echo ""

echo "Common hardware events:"
echo "  cpu-cycles (alias: cycles)"
echo "  instructions"
echo "  cache-references"
echo "  cache-misses"
echo "  branch-instructions (alias: branches)"
echo "  branch-misses"
echo "  bus-cycles"
echo "  ref-cycles"
echo ""
echo "Note: Requires root or CAP_PERFMON capability."