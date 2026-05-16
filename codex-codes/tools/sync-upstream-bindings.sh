#!/usr/bin/env bash
# Refresh the pinned snapshot of upstream codex protocol source files used
# by `tests/protocol_name_conformance.rs`. The snapshot is the source of
# truth for wire field names — bumping the pinned tag here is the act of
# committing to a new upstream protocol revision.
#
# Usage: tools/sync-upstream-bindings.sh [TAG]
#   TAG defaults to the version this script was last synced to.

set -euo pipefail

TAG="${1:-rust-v0.130.0}"
REPO="openai/codex"
SRC_BASE="codex-rs/app-server-protocol/src/protocol"

# Files we care about, expressed as paths under SRC_BASE. They mirror the
# upstream layout on disk so destination structure matches. Add more here
# as the conformance mapping grows.
FILES=(
    v1.rs
    v2/permissions.rs
    v2/notification.rs
    v2/item.rs
    v2/thread.rs
    v2/thread_data.rs
    v2/turn.rs
)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEST_ROOT="$SCRIPT_DIR/../tests/test_data/upstream"
mkdir -p "$DEST_ROOT/v2"

# Resolve the tag to a commit SHA up front so every file fetch lands on a
# single, reproducible upstream revision.
TAG_SHA="$(gh api "repos/$REPO/git/refs/tags/$TAG" --jq '.object.sha')"
COMMIT_SHA="$(gh api "repos/$REPO/git/tags/$TAG_SHA" --jq '.object.sha' 2>/dev/null || echo "$TAG_SHA")"
echo "Syncing $REPO @ $TAG (commit $COMMIT_SHA)"

for f in "${FILES[@]}"; do
    echo "  fetching $SRC_BASE/$f"
    mkdir -p "$(dirname "$DEST_ROOT/$f")"
    gh api "repos/$REPO/contents/$SRC_BASE/$f?ref=$COMMIT_SHA" --jq '.content' \
        | base64 -d > "$DEST_ROOT/$f"
done

cat > "$DEST_ROOT/PINNED_TAG.txt" <<EOF
repo:   $REPO
tag:    $TAG
commit: $COMMIT_SHA
synced: $(date -u +%Y-%m-%dT%H:%M:%SZ)
EOF

echo "Done. Pinned metadata written to $DEST_ROOT/PINNED_TAG.txt"
