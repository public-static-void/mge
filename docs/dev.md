# Developer Guide

This document explains how to set up your environment and run tests.
It mirrors the CI pipeline so all checks can be reproduced locally.

---

## Prerequisites

- **Rust** (latest stable, with `cargo`)
- **Python 3.8+** (for Python bridge/tests)
- **Lua 5.1 or LuaJIT** (for Lua scripting/tests)
- **GCC/Clang** (for C ABI plugins)
- **Maturin** (`pip install maturin`) for Python bindings
- **pytest** (`pip install pytest`) for Python tests

---

## Linting

### Rust Formatting and Linting

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Building C Plugins

Build all C plugins:

```sh
for src in plugins/*.c; do
  base=$(basename "$src" .c)
  gcc -Iengine -shared -fPIC "$src" -o "plugins/lib${base}.so"
done
```

---

## Build Rust Plugins

```sh
cargo run -p xtask -- build-plugins
```

---

## Running Tests

### Rust Unit/Integration Tests

Build and run all Rust tests:

```sh
cargo test --all
```

---

### Lua Scripting Tests

Run all Lua scripting tests:

```sh
./run_lua_tests.sh
```

---

### Python Scripting Tests

1. Create and activate a virtual environment:

   ```sh
   cd engine_py
   python3 -m venv .venv
   source .venv/bin/activate
   pip install --upgrade pip
   pip install maturin pytest
   ```

2. Build and test the Python bindings:

   ```sh
   maturin develop
   pytest
   cd ..
   ```

---

## Schema Validation

To validate all component schemas:

```sh
cargo run --bin schema_validator -- engine/assets/schemas/
```

---

## Adding Components, Systems, or Plugins

- Add or edit schemas in `engine/assets/schemas/`.
- Add Rust systems in `engine_core/systems/`.
- Expose new APIs in scripting bridges as needed.
- Build C plugins in `plugins/` as shown above.
