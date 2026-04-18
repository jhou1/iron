#!/bin/bash
set -euo pipefail

FILE=$(jq -r '.tool_input.file_path // .tool_response.filePath // empty')

# Skip if the edited file is one of the version files (avoid infinite loop)
case "$FILE" in
  */Cargo.toml|*/dashboard.rs) exit 0 ;;
esac

# Skip non-source files
case "$FILE" in
  *.rs|*.toml) ;;
  *) exit 0 ;;
esac

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CARGO="$PROJECT_DIR/Cargo.toml"
DASHBOARD="$PROJECT_DIR/src/tui/dashboard.rs"

# Extract current version from Cargo.toml
CURRENT=$(grep -m1 '^version' "$CARGO" | sed 's/.*"\(.*\)".*/\1/')
MAJOR=$(echo "$CURRENT" | cut -d. -f1)
MINOR=$(echo "$CURRENT" | cut -d. -f2)
PATCH=$(echo "$CURRENT" | cut -d. -f3)
NEW_PATCH=$((PATCH + 1))
NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"

# Update Cargo.toml
sed -i '' "s/^version = \"$CURRENT\"/version = \"$NEW_VERSION\"/" "$CARGO"

# Update dashboard.rs
sed -i '' "s/v$CURRENT/v$NEW_VERSION/" "$DASHBOARD"

echo "{\"suppressOutput\": true}"
