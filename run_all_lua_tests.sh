#!/bin/sh
set -e

PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
TEST_DIR="$PROJECT_ROOT/engine/scripts/lua/tests"
LUA_PATH="$TEST_DIR/?.lua;;"

# Use mge-cli if available, otherwise fallback to cargo run
if command -v mge-cli >/dev/null 2>&1; then
	RUNNER="mge-cli"
else
	RUNNER="cargo run --quiet --bin mge-cli --"
fi

for f in "$TEST_DIR"/test_*.lua; do
	echo "Running $f"
	LUA_PATH="$LUA_PATH" $RUNNER "$f"
done
