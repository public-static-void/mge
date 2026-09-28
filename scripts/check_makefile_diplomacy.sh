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

if [ "$FAILURES" -ne 0 ]; then
  echo "$FAILURES check(s) FAILED"
  exit 1
fi
echo "All M1 checks passed"
