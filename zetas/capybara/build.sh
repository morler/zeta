#!/bin/bash
# Capybara DB build script
# Assembles multi-file Zeta sources into single-file for zetac compiler
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
OUT="${1:-/tmp/capybara}"
SRC="$DIR/src"
BUILD="$DIR/build.z"

# Build order: base layers → client sessions → storage layer → VSR → main
# Every input is required; a missing file must fail loudly rather than emit a
# truncated build.z (see issue #19).
SOURCES=(
  os.z xxh64.z runtime.z ledger.z client_session.z bloom.z memtable.z
  sstable.z manifest.z lsm.z vsr.z main.z
)
MISSING=0
for f in "${SOURCES[@]}"; do
  if [[ ! -f "$SRC/$f" ]]; then
    echo "error: missing source $SRC/$f" >&2
    MISSING=1
  fi
done
if (( MISSING )); then
  echo "error: refusing to assemble an incomplete build.z" >&2
  exit 1
fi

# Assemble into a temp file and only replace build.z on success.
TMP="$(mktemp "$DIR/.build.z.XXXXXX")"
trap 'rm -f "$TMP"' EXIT

emit() { echo "// === $1 ==="; }
# strip "use os;" lines; grep exits 1 when nothing matches, which set -e would
# otherwise treat as a failure
strip_use_os() { grep -v "^use os;" "$1" || true; }

{
  emit os.z;               cat "$SRC/os.z"; echo ""
  emit xxh64.z;            cat "$SRC/xxh64.z"; echo ""
  emit runtime.z;          strip_use_os "$SRC/runtime.z"; echo ""
  emit ledger.z;           strip_use_os "$SRC/ledger.z"; echo ""
  emit client_session.z;   cat "$SRC/client_session.z"; echo ""
  emit bloom.z;            cat "$SRC/bloom.z"; echo ""
  emit memtable.z;         cat "$SRC/memtable.z"; echo ""
  emit sstable.z;          cat "$SRC/sstable.z"; echo ""
  emit manifest.z;         cat "$SRC/manifest.z"; echo ""
  emit lsm.z;              cat "$SRC/lsm.z"; echo ""
  emit vsr.z;              strip_use_os "$SRC/vsr.z"; echo ""
  emit main.z;             strip_use_os "$SRC/main.z"
} > "$TMP"

mv "$TMP" "$BUILD"
trap - EXIT

echo "=== Assembled build.z ==="
wc -l "$BUILD"

echo "=== Compiling ==="
cd /home/zeta/.openclaw/workspace/zeta
export PATH="/home/zeta/.local/bin:$PATH"
bin/zetac "$BUILD" -o "$OUT" 2>&1
echo "=== Done ==="
