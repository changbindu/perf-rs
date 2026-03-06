#!/bin/bash
# Example: List available performance events
# 
# This example demonstrates how to list available performance events
# on your system using perf-rs.

set -e

echo "=== perf-rs list examples ==="
echo ""

# Basic list - show all available events
echo "1. List all available events:"
echo "   sudo perf-rs list"
echo ""

# Filter events by name
echo "2. Filter events by name (cache-related events):"
echo "   sudo perf-rs list --filter cache"
echo ""

# Show detailed event information
echo "3. Show detailed event information:"
echo "   sudo perf-rs list --detailed"
echo ""

# Combine filter and detailed view
echo "4. Filter and show details:"
echo "   sudo perf-rs list --filter branch --detailed"
echo ""

echo "Note: Listing events does not require elevated privileges on most systems."
echo "      However, some systems may require root access depending on perf_event_paranoid settings."