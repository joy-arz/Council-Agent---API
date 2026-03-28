#!/bin/bash
# cleanup_worktrees.sh - Remove old enclave worktree sessions
# 
# This script removes session directories older than a specified number of days
# to prevent disk space accumulation from temporary worktrees.
#
# Usage: ./scripts/cleanup_worktrees.sh [days_to_keep]
# Example: ./scripts/cleanup_worktrees.sh 7  (keeps sessions from last 7 days)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WORKTREES_DIR="$PROJECT_ROOT/.enclave_worktrees"
DAYS_TO_KEEP="${1:-7}"  # Default: keep sessions from last 7 days

echo "=== Enclave Worktree Cleanup ==="
echo "Project root: $PROJECT_ROOT"
echo "Worktrees directory: $WORKTREES_DIR"
echo "Keeping sessions from last $DAYS_TO_KEEP days"
echo ""

if [ ! -d "$WORKTREES_DIR" ]; then
    echo "✓ No worktrees directory found. Nothing to clean."
    exit 0
fi

# Count sessions before cleanup
BEFORE_COUNT=$(find "$WORKTREES_DIR" -maxdepth 1 -type d -name "session_*" | wc -l | tr -d ' ')
echo "Current sessions: $BEFORE_COUNT"

# Find and remove old sessions
REMOVED_COUNT=0
while IFS= read -r -d '' session_dir; do
    if [ -n "$session_dir" ]; then
        echo "Removing: $(basename "$session_dir")"
        rm -rf "$session_dir"
        ((REMOVED_COUNT++)) || true
    fi
done < <(find "$WORKTREES_DIR" -maxdepth 1 -type d -name "session_*" -mtime +$DAYS_TO_KEEP -print0 2>/dev/null)

# Count sessions after cleanup
AFTER_COUNT=$(find "$WORKTREES_DIR" -maxdepth 1 -type d -name "session_*" | wc -l | tr -d ' ')

echo ""
echo "=== Cleanup Summary ==="
echo "Removed: $REMOVED_COUNT sessions"
echo "Remaining: $AFTER_COUNT sessions"

if [ "$REMOVED_COUNT" -eq 0 ]; then
    echo "✓ No old sessions to remove"
else
    echo "✓ Cleanup complete!"
fi

exit 0
