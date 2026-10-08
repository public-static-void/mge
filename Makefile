# ====== PHONY TARGETS ======
.PHONY: all build-plugins build-c-plugins build-c-plugins-if-needed build-wasm-tests build-all \
	test test-rust test-python test-lua test-wasm test-all \
	setup-python build-python build-wheel clean validate-schema help \
	lint fmt fmt-check run-cli run-demo lint-docs

# ====== CONFIGURABLE VARIABLES ======
SCHEMA_DIR := engine/assets/schemas
# LUA_FILTER forwards module/function filters to the Lua test runner.
# Empty (default) = full suite; e.g. LUA_FILTER=test_hex_zlevel runs one module.
LUA_FILTER :=

# ====== HELP TARGET ======
help:
	@echo "Available targets:"
	@echo "  make all              - Build everything (validates schemas first)"
	@echo "  make build-plugins    - Build Rust plugins via xtask"
	@echo "  make build-c-plugins  - Build C plugins via xtask"
	@echo "  make build-c-plugins-if-needed - Reuse C artifact if fresh, else rebuild"
	@echo "  make build-wasm-tests - Build WASM guest test modules via xtask"
	@echo "  make build-all        - Build all plugins via xtask"
	@echo "  make test             - Run all tests and validate schemas"
	@echo "  make test-rust        - Run Rust tests"
	@echo "  make test-python      - Run Python tests (with venv/maturin setup)"
	@echo "  make test-lua         - Run Lua tests"
	@echo "  make test-wasm        - Run WASM tests"
	@echo "  make test-all         - Run all tests and validate schemas"
	@echo "  make setup-python     - Set up Python venv and dependencies"
	@echo "  make build-python     - Build Python Rust extension with maturin"
	@echo "  make build-wheel      - Build Python wheel for distribution"
	@echo "  make clean            - Clean Rust build artifacts"
	@echo "  make validate-schema  - Validate game/data schemas"
	@echo "  make help             - Show this help"
	@echo "  make lint             - Run clippy lint gate (-D warnings)"
	@echo "  make fmt              - Apply formatting (cargo fmt)"
	@echo "  make fmt-check        - Verify formatting (cargo fmt --check)"
	@echo '  make run-cli          - Run game CLI (forward args via ARGS="...")'
	@echo "  make run-demo         - Run viewport demo"
	@echo "  make lint-docs        - Fail on plain-cargo regressions in user-facing docs"

# ====== LINT / FORMAT TARGETS ======
lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

# ====== RUN TARGETS ======
run-cli:
	cargo run --bin mge_cli -- $(ARGS)

run-demo:
	cargo run --example viewport_demo -p engine_core

# ====== DOCS REGRESSION GUARD ======
# User-facing docs must use make targets, never plain cargo (M2/R011).
lint-docs:
	@if grep -rnE 'cargo (run|build|test)' AGENTS.md README.md docs/dev.md docs/plugin_abi.md engine/tools/schema_validator/README.md; then \
		echo "plain-cargo found in user-facing docs"; exit 1; \
	fi

# ====== SCHEMA VALIDATION ======
# Extra validator args (alternate path, flags) forward via ARGS; defaults to SCHEMA_DIR.
validate-schema:
	cargo run --bin schema_validator --release -- $(if $(ARGS),$(ARGS),$(SCHEMA_DIR))

# ====== RUST, C & WASM BUILD TARGETS ======
build-plugins:
	cargo run -p xtask -- build-plugins

build-c-plugins:
	cargo run -p xtask -- build-c-plugins

# Content-aware logged guard (project/4): reuses the downloaded c-plugins
# artifact when every expected .so exists and is newer than its .c source,
# otherwise delegates to build-c-plugins. test-rust/test-lua depend on this
# target instead of a silent conditional prerequisite, so consumer logs
# always carry an explicit "reusing ..." or "rebuilding ..." line (CI=$CI).
# A path mismatch degrades to a logged rebuild, never a silent one.
C_PLUGIN_SRCS := $(wildcard plugins/*/*.c)

build-c-plugins-if-needed:
	@status=0; \
	for src in $(C_PLUGIN_SRCS); do \
		out=$$(dirname "$$src")/lib$$(basename "$$src" .c).so; \
		if [ ! -f "$$out" ]; then echo "c-plugins artifact missing: $$out"; status=1; \
		elif [ "$$src" -nt "$$out" ]; then echo "c-plugins source newer than artifact: $$src"; status=1; \
		fi; \
	done; \
	if [ "$$status" -eq 0 ]; then \
		echo "reusing c-plugins artifact (CI=$${CI:-unset})"; \
	else \
		echo "rebuilding c-plugins (CI=$${CI:-unset}) ..."; \
		$(MAKE) build-c-plugins; \
	fi

build-wasm-tests:
	cargo run -p xtask -- build-wasm-tests

build-all:
	cargo run -p xtask -- build-all

# ====== RUST TEST TARGET (sharded per crate: one tool-timeout budget per shard) ======
# CI guard: consumer jobs reuse the downloaded c-plugins artifact through the
# logged build-c-plugins-if-needed target below (explicit reuse/rebuild line
# in the consumer log). Fresh-clone local runs (artifacts absent or stale)
# rebuild C plugins from source via the same target.
test-rust: build-c-plugins-if-needed
	cargo test -p engine_core
	cargo test -p engine_macros
	cargo test -p engine_py
	cargo test -p engine_lua
	cargo test -p engine_wasm
	cargo test -p schema_validator
	cargo test -p rust_test_plugin
	cargo test -p xtask

# ====== PYTHON SETUP, BUILD, AND TEST TARGETS ======

# Set up Python venv and install dependencies (idempotent)
setup-python:
	@echo "Setting up Python venv and installing dependencies..."
	@if [ ! -d engine_py/.venv ]; then \
		cd engine_py && python3 -m venv .venv; \
		fi
	@cd engine_py && . .venv/bin/activate && pip install -U pip && \
		[ -f requirements.txt ] && pip install -r requirements.txt || true

# Build/install Rust extension into venv using maturin (idempotent)
build-python: setup-python
	@command -v maturin >/dev/null 2>&1 || { echo >&2 "maturin is not installed. Aborting."; exit 1; }
	@echo "Building Python Rust extension with maturin..."
	@cd engine_py && . .venv/bin/activate && maturin develop --release

# Build Python wheel for distribution (standalone, not part of dev workflow)
build-wheel:
	@command -v maturin >/dev/null 2>&1 || { echo >&2 "maturin is not installed. Aborting."; exit 1; }
	@echo "Building Python wheel with maturin..."
	@cd engine_py && maturin build --release

# Run Python tests (always runs setup and build first)
# Same logged guard as test-rust/test-lua: the Python suite needs the C .so
# on LD_LIBRARY_PATH, so artifact reuse or rebuild is explicit in the log.
test-python: build-c-plugins-if-needed build-python
	@echo "Running Python tests..."
	@cd engine_py && . .venv/bin/activate && pytest

# ====== LUA TEST TARGET ======
# Same logged guard as test-rust: artifact reuse or rebuild is explicit in the log.
test-lua: build-c-plugins-if-needed
	@echo "Running Lua tests..."
	cargo build --package engine_lua --bin mge_lua_test_runner
	./run_lua_tests.sh $(LUA_FILTER)

# ====== WASM TEST TARGET ======
test-wasm: build-wasm-tests
	cargo test -p engine_wasm

# ====== AGGREGATED TEST TARGETS ======
test-all: validate-schema test-rust test-python test-lua test-wasm
test: test-all

# ====== CLEAN TARGET ======
clean:
	cargo clean

# ====== DEFAULT TARGET ======
all: validate-schema build-all
