#!/bin/bash
# Example: Dump trace data with perf-rs script
#
# This example demonstrates how to export raw trace data
# from perf.data files.

set -e

echo "=== perf-rs script examples ==="
echo ""

# Basic script dump
echo "1. Dump perf.data in text format:"
echo "   sudo perf-rs script"
echo ""

# Specify input file
echo "2. Dump from custom data file:"
echo "   sudo perf-rs script --input my_profile.data"
echo ""

# Show call chains
echo "3. Show call chains (stack traces):"
echo "   sudo perf-rs script --callchain"
echo ""

# Output as JSON
echo "4. Export as JSON:"
echo "   sudo perf-rs script --format json"
echo ""

# Combine callchain with JSON
echo "5. Export call chains as JSON:"
echo "   sudo perf-rs script --callchain --format json"
echo ""

# Save to file
echo "6. Save trace to file:"
echo "   sudo perf-rs script > trace.txt"
echo ""

# Process for external analysis
echo "7. Export JSON for analysis:"
echo "   sudo perf-rs script --format json --callchain > trace.json"
echo ""

echo "Output formats:"
echo "  text  - Human-readable format (default)"
echo "  json  - JSON for programmatic processing"
echo ""
echo "Call chain options:"
echo "  --callchain  - Include stack traces for each sample"
echo ""
echo "Note: Useful for detailed analysis or integration with other tools."