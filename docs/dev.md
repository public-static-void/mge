# Developer Guide

This document explains how to set up your environment, build the project, run tests, and validate data.
**All major workflows are automated via the project Makefile.**
This mirrors the CI pipeline so all checks can be reproduced locally.

---

## Prerequisites

- **Rust nightly-2026-06-01** (edition 2024 requires nightly)
  ```sh
  rustup toolchain install nightly-2026-06-01
  rustup default nightly-2026-06-01
  ```
- **Python 3.8+** (for Python scripting/tests)
- **Lua 5.1 or LuaJIT** (for Lua scripting/tests)
- **GCC/Clang** (for C ABI plugins)
- **Maturin** (`pip install maturin`) for Python bindings
- **pytest** (`pip install pytest`) for Python tests
- **WASM** — built via xtask:
  ```sh
  cargo run -p xtask -- build-wasm-tests
  ```

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

## Development Workflow

### Recommended Iteration Loop

The standard technical iteration cycle for MGE development:

1. **Validate schemas** — `make validate-schema` checks all JSON component schemas.
2. **Build** — `make all` validates schemas and builds Rust crates, C plugins, and WASM tests.
3. **Test** — `make test` runs schema validation + Rust + Python + Lua tests. Use individual targets (`make test-rust`, `make test-python`, `make test-lua`) for faster feedback on backend-specific changes.
4. **Lint** — `cargo fmt --all --check` and `cargo clippy --all-targets --all-features -- -D warnings`.

CI enforces this sequence: `validate-schema → build-c-plugins → build-wasm-tests → build-all → test-rust → test-python → test-lua`.

### Branching

Feature branches branch from `main` and are merged back via pull requests. Follow the semantic commit format: `<type>(<scope>): <subject>` (e.g., `feat(core): add spatial index`, `fix(lua): correct entity lookup`).

---

## Linting

### Rust Formatting and Linting

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Engine Entrypoints

| Binary | Crate | Purpose |
|---|---|---|
| `mge_cli` | `engine_lua` | Main game CLI — run Lua scripts directly or via `--mod <name>` for mod-based content |
| `mge_lua_test_runner` | `engine_lua` | Lua integration test harness |
| `schema_validator` | `schema_validator` | Validate all JSON schemas in `engine/assets/schemas/` |
| `xtask` | `xtask` | Plugin build/deploy orchestrator (build-plugins, build-c-plugins, build-wasm-tests, build-all) |

---

## Adding Components, Systems, or Plugins

- Add or edit schemas in `engine/assets/schemas/`.
- Add Rust systems in `engine_core/systems/`.
- Expose new APIs in scripting bridges as needed.
- Build C plugins in `plugins/` (see Makefile).
