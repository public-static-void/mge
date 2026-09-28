#!/usr/bin/env bash
# Standing verification suite for the Makefile-diplomacy session.
# Created in M1 (R001-R004 / AC001-AC004 checks); extended in place by M2-M5.
# Usage: scripts/check_makefile_diplomacy.sh [--skip-smoke]
#   --skip-smoke  recipe-level checks only (no `make run-cli` build+run)
set -euo pipefail

SKIP_SMOKE=0
for arg in "$@"; do
  case "$arg" in
    --skip-smoke) SKIP_SMOKE=1 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

FAILURES=0
fail() { echo "FAIL: $1"; FAILURES=$((FAILURES + 1)); }
pass() { echo "PASS: $1"; }

# --- AC001: `make lint` exists and runs clippy with -D warnings ---
if grep -q '^lint:' Makefile \
  && sed -n '/^lint:/,/^[^[:space:]]/p' Makefile | grep -q -- '-D warnings'; then
  pass "AC001 make lint runs clippy with -D warnings"
else
  fail "AC001 make lint missing or without -D warnings"
fi

# --- AC002: `make fmt` / `make fmt-check` exist; fmt-check green on this tree ---
if grep -q '^fmt:[[:space:]]*$' Makefile && grep -q '^fmt-check:' Makefile; then
  pass "AC002 make fmt and make fmt-check targets exist"
else
  fail "AC002 make fmt / make fmt-check targets missing"
fi
if make fmt-check >/dev/null 2>&1; then
  pass "AC002 make fmt-check exits 0"
else
  fail "AC002 make fmt-check exits nonzero"
fi

# --- AC003: run-cli forwards ARGS to mge_cli; run-demo targets viewport demo ---
if grep -q '^run-cli:' Makefile \
  && sed -n '/^run-cli:/,/^[^[:space:]]/p' Makefile | grep -q 'mge_cli' \
  && sed -n '/^run-cli:/,/^[^[:space:]]/p' Makefile | grep -q '$(ARGS)'; then
  pass "AC003 make run-cli forwards ARGS to mge_cli"
else
  fail "AC003 make run-cli missing or without ARGS forwarding to mge_cli"
fi
if grep -q '^run-demo:' Makefile \
  && sed -n '/^run-demo:/,/^[^[:space:]]/p' Makefile | grep -q 'viewport_demo'; then
  pass "AC003 make run-demo targets viewport demo"
else
  fail "AC003 make run-demo missing or not targeting viewport demo"
fi
if [ "$SKIP_SMOKE" -eq 0 ]; then
  if make run-cli ARGS="--help" >/dev/null 2>&1; then
    pass "AC003 make run-cli ARGS=\"--help\" smoke-run green"
  else
    fail "AC003 make run-cli ARGS=\"--help\" smoke-run failed"
  fi
else
  echo "SKIP: AC003 run-cli smoke-run (--skip-smoke)"
fi

# --- AC004/NFR001: every .PHONY token appears in `make help` output ---
# .PHONY spans backslash-continuation lines; join them before tokenizing.
PHONY_TOKENS=$(awk 'BEGIN{p=0} /^\.PHONY:/{p=1; sub(/^\.PHONY:[[:space:]]*/,"")} p{if(/\\$/){sub(/\\$/,""); printf "%s ",$0; next} else {print $0; p=0}}' Makefile | tr -s '[:space:]' '\n' | grep -v '^$' | sort -u)
HELP_TEXT=$(make help 2>/dev/null || true)
MISSING=""
while IFS= read -r token; do
  [ -z "$token" ] && continue
  if ! printf '%s\n' "$HELP_TEXT" | grep -q -w -- "$token"; then
    MISSING="$MISSING $token"
  fi
done <<< "$PHONY_TOKENS"
if [ -z "$MISSING" ]; then
  pass "AC004 every .PHONY token appears in make help output"
else
  fail "AC004 tokens missing from make help output:$MISSING"
fi

# --- M2/AC008: test-rust sharded per crate, no bare `cargo test --all` ---
RUST_RECIPE=$(sed -n '/^test-rust:/,/^[^[:space:]]/p' Makefile)
if printf '%s\n' "$RUST_RECIPE" | grep -q 'cargo test --all'; then
  fail "AC008 test-rust still contains bare cargo test --all"
else
  pass "AC008 test-rust contains no bare cargo test --all"
fi
MISSING_SHARDS=""
for member in engine_core engine_macros engine_py engine_lua engine_wasm schema_validator rust_test_plugin xtask; do
  if ! printf '%s\n' "$RUST_RECIPE" | grep -q "cargo test -p $member"; then
    MISSING_SHARDS="$MISSING_SHARDS $member"
  fi
done
if [ -z "$MISSING_SHARDS" ]; then
  pass "AC008 test-rust shards cover all 8 workspace members"
else
  fail "AC008 test-rust missing shards:$MISSING_SHARDS"
fi

# --- M2/AC009: structural artifact ordering ---
if grep -q '^test-wasm:.*build-wasm-tests' Makefile; then
  pass "AC009 test-wasm depends on build-wasm-tests"
else
  fail "AC009 test-wasm missing build-wasm-tests dependency"
fi
if grep -q '^test-rust:.*build-c-plugins' Makefile; then
  pass "AC009 test-rust depends on build-c-plugins"
else
  fail "AC009 test-rust missing build-c-plugins dependency"
fi
if grep -q '^test-lua:.*build-c-plugins' Makefile; then
  pass "AC009 test-lua depends on build-c-plugins"
else
  fail "AC009 test-lua missing build-c-plugins dependency"
fi

# --- M2/AC011(R012): test/test-all/all semantics preserved ---
if grep -q '^test-all:' Makefile \
  && sed -n '/^test-all:/p' Makefile | grep -q 'validate-schema' \
  && sed -n '/^test-all:/p' Makefile | grep -q 'test-rust' \
  && sed -n '/^test-all:/p' Makefile | grep -q 'test-python' \
  && sed -n '/^test-all:/p' Makefile | grep -q 'test-lua' \
  && sed -n '/^test-all:/p' Makefile | grep -q 'test-wasm'; then
  pass "AC011 test-all still expands to validate-schema plus all four suites"
else
  fail "AC011 test-all semantics changed"
fi
if grep -q '^test:.*test-all' Makefile \
  && grep -q '^all:.*validate-schema' Makefile \
  && grep -q '^all:.*build-all' Makefile; then
  pass "AC011 test and all aliases preserved"
else
  fail "AC011 test/all alias semantics changed"
fi

# --- M2/R011: lint-docs guard exists and its verdict matches docs state ---
if grep -q '^lint-docs:' Makefile \
  && sed -n '/^lint-docs:/,/^[^[:space:]]/p' Makefile | grep -q 'AGENTS.md' \
  && sed -n '/^lint-docs:/,/^[^[:space:]]/p' Makefile | grep -q 'cargo (run|build|test)'; then
  pass "R011 lint-docs target greps user-facing docs for plain-cargo"
else
  fail "R011 lint-docs target missing or mis-scoped"
fi
# Guard-consistency: the guard must fail exactly when plain-cargo is present.
# Red while the M3 docs rewrite is pending, green after — correct either way.
if grep -rnE 'cargo (run|build|test)' AGENTS.md README.md docs/dev.md docs/plugin_abi.md >/dev/null 2>&1; then
  DOCS_DIRTY=1
else
  DOCS_DIRTY=0
fi
if make lint-docs >/dev/null 2>&1; then
  LINTDOCS_GREEN=1
else
  LINTDOCS_GREEN=0
fi
if { [ "$DOCS_DIRTY" -eq 1 ] && [ "$LINTDOCS_GREEN" -eq 0 ]; } \
  || { [ "$DOCS_DIRTY" -eq 0 ] && [ "$LINTDOCS_GREEN" -eq 1 ]; }; then
  pass "R011 lint-docs verdict matches docs state (dirty=$DOCS_DIRTY green=$LINTDOCS_GREEN)"
else
  fail "R011 lint-docs verdict contradicts docs state (dirty=$DOCS_DIRTY green=$LINTDOCS_GREEN)"
fi

# --- M3/AC005-AC007: docs use make forms, require-shim note present ---
if grep -rnE 'cargo (run|build|test)' AGENTS.md README.md docs/dev.md docs/plugin_abi.md >/dev/null 2>&1; then
  fail "AC005 plain-cargo user instruction remains in AGENTS.md/README.md/docs/dev.md/docs/plugin_abi.md"
else
  pass "AC005 zero plain-cargo in user-facing docs"
fi
if grep -q 'require.*shim' AGENTS.md && grep -q 'mge_lua_test_runner' AGENTS.md; then
  pass "AC007/R017 AGENTS.md carries the Lua require-shim note"
else
  fail "AC007/R017 AGENTS.md require-shim note missing"
fi
if grep -qE 'cargo (clippy|fmt)|pytest tests/' AGENTS.md; then
  fail "AC006 raw clippy/fmt/pytest instruction remains in AGENTS.md"
else
  pass "AC006 no raw clippy/fmt/pytest instruction in AGENTS.md"
fi
if grep -q 'make test' docs/dev.md \
  && grep -q 'test-rust' docs/dev.md \
  && grep -q 'test-python' docs/dev.md \
  && grep -q 'test-lua' docs/dev.md \
  && grep -q 'test-wasm' docs/dev.md; then
  pass "AC007 docs/dev.md make test row names all four suites"
else
  fail "AC007 docs/dev.md make test row missing a suite"
fi

# --- M3/AC010 Branch B: docs CI sequence matches as-built ci.yml jobs ---
for job in fmt clippy validate-schema build-c-plugins build-wasm-tests test-rust test-lua test-python; do
  if ! grep -q "$job" .github/workflows/ci.yml; then
    fail "AC010 ci.yml job $job missing (unexpected as-built drift)"
  fi
done
if grep -E '^\s+test-wasm:' .github/workflows/ci.yml >/dev/null 2>&1; then
  fail "AC010 Branch B violated: ci.yml unexpectedly gained a test-wasm job"
else
  pass "AC010 Branch B: ci.yml has no test-wasm job"
fi
if grep -E 'test-rust → test-python → test-lua → test-wasm$|build-all → test-rust → test-python → test-lua → test-wasm' AGENTS.md >/dev/null 2>&1; then
  fail "AC010 stale Required Command Order still claims test-wasm as a CI step"
else
  pass "AC010 Required Command Order carries no stale test-wasm CI claim"
fi
if grep -q 'test-rust.*needs.*validate-schema.*build-c-plugins.*build-wasm-tests' docs/dev.md \
  && grep -q 'no standalone.*test-wasm.*CI job' docs/dev.md; then
  pass "AC010 docs/dev.md CI sequence matches as-built ci.yml jobs"
else
  fail "AC010 docs/dev.md CI sequence does not match as-built ci.yml jobs"
fi

# --- M4/AC012: genre traceability notes name store, system, war-gate, bridges, tests ---
for genre in grand-strategy 4x; do
  GENRE_FILE="docs/genres/$genre.md"
  for token in DiplomacyState DiplomacySystem EnemyBehaviorSystem; do
    if grep -q "$token" "$GENRE_FILE"; then
      pass "AC012 $genre.md names $token"
    else
      fail "AC012 $genre.md missing $token"
    fi
  done
  for bridge in engine_lua/src/lua_api/diplomacy.rs engine_py/src/python_api/diplomacy.rs engine_wasm/src/host_api/diplomacy.rs; do
    if grep -q "$bridge" "$GENRE_FILE"; then
      pass "AC012 $genre.md traces bridge $bridge"
    else
      fail "AC012 $genre.md missing bridge trace $bridge"
    fi
  done
  if grep -q 'test_diplomacy' "$GENRE_FILE"; then
    pass "AC012 $genre.md traces diplomacy test artifacts"
  else
    fail "AC012 $genre.md missing diplomacy test trace"
  fi
done

# --- M4/AC013: strategy modes registered in schemas + game.toml ---
for schema in engine/assets/schemas/diplomacy.json engine/assets/schemas/treaty.json; do
  for mode in grand-strategy 4x; do
    if python3 -c "import json,sys; sys.exit(0 if '$mode' in json.load(open('$schema')).get('modes', []) else 1)"; then
      pass "AC013 $schema modes contains $mode"
    else
      fail "AC013 $schema modes missing $mode"
    fi
  done
done
for mode in grand-strategy 4x; do
  if grep -q "\"$mode\"" game.toml; then
    pass "AC013 game.toml allowed_modes contains $mode"
  else
    fail "AC013 game.toml allowed_modes missing $mode"
  fi
done
if make validate-schema >/dev/null 2>&1; then
  pass "AC013 make validate-schema green with strategy modes"
else
  fail "AC013 make validate-schema red with strategy modes"
fi

# --- M4/R016 scope guard: no diplomacy core diffs in this milestone ---
if git diff --quiet -- engine/core/src/diplomacy.rs engine/core/src/systems/ engine_lua/src/lua_api/diplomacy.rs engine_py/src/python_api/diplomacy.rs engine_wasm/src/host_api/diplomacy.rs 2>/dev/null; then
  pass "R016 zero diplomacy core/system/bridge diffs"
else
  fail "R016 diplomacy core/system/bridge diff detected"
fi

if [ "$FAILURES" -ne 0 ]; then
  echo "$FAILURES check(s) FAILED"
  exit 1
fi
echo "All Makefile-diplomacy checks passed"
