# Developer Guide

This document explains how to set up your environment, build the project, run tests, and validate data.
**All major workflows are automated via the project Makefile.**
This mirrors the CI pipeline so all checks can be reproduced locally.

---

## Prerequisites

- **Rust** (latest stable, with `cargo`)
- **Python 3.8+** (for Python scripting/tests)
- **Lua 5.1 or LuaJIT** (for Lua scripting/tests)
- **GCC/Clang** (for C ABI plugins)
- **Maturin** (`pip install maturin`) for Python bindings
- **pytest** (`pip install pytest`) for Python tests
- **WebAssembly** (`rustup target add wasm32-unknown-unknown`) for WASM

---

## Using the Makefile

All major build, test, and validation tasks are automated via the project `Makefile`.
**This is the recommended way to work with the project, as it mirrors the CI pipeline and guarantees a reproducible developer experience.**

### Common Makefile Targets

| Target                 | Description                                            |
| ---------------------- | ------------------------------------------------------ |
| `make all`             | Build everything (validates schemas first)             |
| `make test`            | Run all tests (Rust, Python, Lua) and validate schemas |
| `make validate-schema` | Validate all component/data schemas                    |
| `make test-python`     | Set up venv, build Rust extension, run Python tests    |
| `make test-rust`       | Build and run all Rust tests                           |
| `make test-lua`        | Run all Lua scripting tests                            |
| `make clean`           | Clean Rust build artifacts                             |
| `make help`            | Show a summary of available targets                    |

### Notes

- The Makefile will automatically set up Python virtual environments, install dependencies, and build Rust and C Plugins as needed.
- All Makefile targets are idempotent and can be safely re-run.
- The Makefile is the **single source of truth** for build and test orchestration; all CI steps use these targets.

---

## Linting

### Rust Formatting and Linting

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Engine Entrypoints

- **Library API (`lib.rs`)**: Use this for embedding, scripting, or integrating the engine in other projects.
- **CLI Runner (`bin/mge_cli.rs`)**: Main entry point for running mods, Lua scripts, or games.
- **Test Runner (`bin/mge_lua_test_runner.rs`)**: Runs all Lua integration tests.
- **Schema Validator (`bin/schema_validator.rs`)**: Validates all component schemas.
- **Plugin Handler (`src/main.rs`)**: Handles plugin protocol (not for direct engine execution).
- **Codegen Tool (`tools/codegen/src/main.rs`)**: Generates Rust/Lua/Python/C code from schemas.

---

## Adding Components, Systems, or Plugins

- Add or edit schemas in `engine/assets/schemas/`.
- Add Rust systems in `engine_core/systems/`.
- Expose new APIs in scripting bridges as needed.
- Build C plugins in `plugins/` (see Makefile).
