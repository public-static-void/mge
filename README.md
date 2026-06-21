# Modular Game Engine (MGE)

**Cross-language game engine** — Rust core with identical scripting APIs in **Lua**, **Python**, and **WASM**, plus **C ABI plugins**.

![CI]({{CI_BADGE_URL}}) · {{LATEST_TAG}}

---

## Quick Start

```sh
git clone <repo> && cd mge
make all                                    # validate schemas → build everything
cargo run --bin mge_cli -- engine/scripts/lua/demos/roguelike_mvp.lua
```

---

## Architecture

9 crates in a Rust workspace (`edition = "2024"`, nightly toolchain):

```
engine_macros (proc-macro)
       ↓
 engine_core  ←  engine_lua  (Lua scripting)
                ←  engine_py   (Python scripting)
                ←  engine_wasm (WASM scripting)
```

| Crate | Role |
|---|---|
| `engine_core` | Pure Rust ECS, simulation, schema loader, plugin ABI — no language deps |
| `engine_lua` | Lua bindings via mlua — CLI binary `mge_cli`, test runner |
| `engine_py` | Python bindings via pyo3 — maturin-based native extension |
| `engine_wasm` | WASM runtime via wasmtime |
| `engine_macros` | Proc-macro crate for `#[component]` attribute |
| `schema_validator` | JSON schema validation tool |
| `xtask` | Build orchestration (plugin deploy, wasm tests, C plugins) |
| `rust_test_plugin` | Test plugin (Rust cdylib) |
| C plugins | `simple_square_plugin`, `simple_hex_plugin`, `simple_province_plugin`, `test_plugin` |

---

## Current State

**Full scripting parity (milestone):** All 3 backends (Lua, Python, WASM) expose ~139 functions each with identical API surfaces:

- Entity lifecycle — `spawn_entity`, `despawn_entity`
- Component CRUD — `set_component`, `get_component`, `remove_component`, `list_components`
- Queries — `get_entities`, `get_entities_with_component`, `count_entities_with_type`
- Map — `add_cell`, `add_neighbor`, `get_all_cells`, `find_path`, `entities_in_cell`
- Movement — `move_entity`, `move_all`
- Combat — `damage_entity`, `damage_all`
- Mode — `set_mode`, `get_mode`, `get_available_modes`
- Simulation — `tick`, `get_turn`, `process_deaths`, `process_decay`
- Worldgen — `register_worldgen_plugin`, `invoke_worldgen_plugin`
- Jobs — full job system (board, query, mutation, events, AI assignment)

---

## Prerequisites

| Dependency | Version |
|---|---|
| Rust | `nightly-2026-06-01` (edition 2024, see `rust-toolchain.toml`) |
| LuaJIT | `libluajit-5.1-dev` + `pkg-config` |
| C compiler | `gcc` + `libjansson-dev` |
| Python | 3.8+ (`maturin`, `pytest`) |

```sh
rustup toolchain install nightly-2026-06-01 && rustup default nightly-2026-06-01
```

---

## Make Targets

| Target | Description |
|---|---|
| `make all` | Validate schemas → build everything |
| `make test` | All tests: schema + Rust + Python + Lua |
| `make validate-schema` | Validate JSON schemas in `engine/assets/schemas/` |
| `make test-rust` | `cargo test --all` |
| `make test-python` | Setup venv → `maturin develop` → `pytest` |
| `make test-lua` | Build test runner → run Lua test suite |
| `make clean` | `cargo clean` |
| `make help` | Show a summary of available targets |

---

## Demo Showcase

### Roguelike MVP

A playable roguelike demonstrating 8+ engine subsystems:

| Subsystem | What it demonstrates |
|---|---|
| Grid map | 40×25 tile map with rooms, corridors, walls |
| Pathfinding | AI enemies navigate via `find_path()` |
| Camera | Viewport follows player |
| ECS + Schemas | Schema-defined Health, Position, Type, Renderable |
| Inventory | Pickup, use, drop items with visible UI |
| Simulation | Structured `tick()` + `get_turn()` game loop |
| Event bus | Combat/death events in message log |
| Death/decay | Corpses, decay timer, loot drops on kill |
| Save/load | 4 save slots with menu-driven save/load |

```sh
cargo run --bin mge_cli -- engine/scripts/lua/demos/roguelike_mvp.lua
```

Controls: `WASD/hjkl` move · `.` wait · `e/g` pickup · `q/u` use · `i` inventory · `d` drop · `S` save · `L` load · `Q` quit

### Viewport Demo

```sh
cargo run --example viewport_demo -p engine_core
```

### Mod Runner

```sh
cargo run --bin mge_cli -- --mod mvp_roguelike
```

Controls: `WASD` move · `e` attack · `.` wait · `q` quit

---

## Documentation

| Doc | Description |
|---|---|
| [docs/dev.md](docs/dev.md) | Developer setup & test guide |
| [docs/idea.md](docs/idea.md) | Architecture and design |
| [docs/api.md](docs/api.md) | Unified scripting API (Lua, Python, WASM) |
| [docs/plugin_abi.md](docs/plugin_abi.md) | C ABI plugin authoring |
| [docs/examples.md](docs/examples.md) | Usage examples (Lua, Python, Rust, C) |
| [docs/worldgen.md](docs/worldgen.md) | Worldgen plugin system |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Project roadmap |
