# ====== PHONY TARGETS ======
.PHONY: all build-plugins build-c-plugins build-wasm-tests build-all \
	test test-rust test-python test-lua test-all \
	setup-python build-python clean validate-schema help

# ====== CONFIGURABLE VARIABLES ======
SCHEMA_DIR := engine/assets/schemas

# ====== HELP TARGET ======
help:
	@echo "Common targets:"
	@echo "  make all             - Build everything (validates schemas first)"
	@echo "  make test            - Run all tests and validate schemas"
	@echo "  make validate-schema - Validate game/data schemas"
	@echo "  make test-python     - Run Python tests (with venv/maturin setup)"
	@echo "  make test-rust       - Run Rust tests"
	@echo "  make test-lua        - Run Lua tests"
	@echo "  make clean           - Clean Rust build artifacts"

# ====== SCHEMA VALIDATION ======
validate-schema:
	cargo run --bin schema_validator --release -- $(SCHEMA_DIR)

# ====== RUST, C & WASM BUILD TARGETS ======
build-plugins:
	cargo run -p xtask -- build-plugins

build-c-plugins:
	cargo run -p xtask -- build-c-plugins

build-wasm-tests:
	cargo run -p xtask -- build-wasm-tests

build-all:
	cargo run -p xtask -- build-all

# ====== RUST TEST TARGET ======
test-rust:
	cargo run -p xtask -- build-all
	cargo test --all

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

# Run Python tests (always runs setup and build first)
test-python: build-python
	@echo "Running Python tests..."
	@cd engine_py && . .venv/bin/activate && pytest

# ====== LUA TEST TARGET ======
test-lua:
	@echo "Running Lua tests..."
	cargo build --package engine_lua --bin mge_lua_test_runner
	./run_lua_tests.sh

# ====== AGGREGATED TEST TARGETS ======
test-all: validate-schema test-rust test-python test-lua
test: test-all

# ====== CLEAN TARGET ======
clean:
	cargo clean

# ====== DEFAULT TARGET ======
all: validate-schema build-all
