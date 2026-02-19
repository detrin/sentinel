#!/bin/bash
# Example action script for Sentinel
# This script receives environment variables from the executor

echo "=== Sentinel Action Script ==="
echo "Switch ID: $SWITCH_ID"
echo "Execution Type: $EXECUTION_TYPE"
echo "Timestamp: $(date)"
echo "=============================="

# Exit with success
exit 0
