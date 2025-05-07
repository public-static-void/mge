# Modular Game Engine (MGE)

MGE is a modular, cross-language game engine blueprint and reference implementation.
It is designed for simulation, games, and rapid prototyping with robust ECS, plugin, and scripting support.

---

## Project Overview

MGE provides:

- A Rust-based core engine with a macro-driven ECS framework.
- Hot-reloadable plugin support and cross-language scripting (Lua, Python, WASM).
- Out-of-the-box Lua scripting bridge for entity/component manipulation and rapid prototyping.
- **Runtime mode switching and mode-specific component enforcement** in both Rust and Lua scripting.
- Mode-specific logic and data (e.g., Colony, Roguelike).
- Schema-driven, versioned component management.
- An architecture designed for tooling, modding, and rapid iteration.

For the full architecture and design blueprint, see [`docs/idea.md`](docs/idea.md).

---

## Repository Structure

```text
engine/
  core/          # ECS core, registry, schema, migration (Rust)
  docs/          # Engine-specific documentation and specs
  assets/        # Game assets and data
  scripts/       # Scripts for gameplay, modding, and tests
  tools/         # Engine tools and utilities
engine_macros/   # Procedural macros for component ergonomics
docs/            # Project-wide docs and blueprints
```

---

## Scripting (Lua)

MGE supports Lua scripting for rapid prototyping, modding, and gameplay logic.

- Scripts can spawn entities, set/get components (e.g., Position, Health), and interact with the ECS.
- **Game systems like movement, health, and turns are also scriptable.**
- Lua scripts are loaded from `engine/scripts/lua/` and can be tested and run as part of the engine.
- You can switch game modes at runtime and only access components valid for the current mode.
- Attempting to set or get a component not available in the current mode will result in an error.

**Available Lua functions:**

| Function             | Description                           |
| -------------------- | ------------------------------------- |
| `spawn_entity()`     | Spawn a new entity, returns entity id |
| `set_component()`    | Set a component on an entity          |
| `get_component()`    | Get a component from an entity        |
| `set_mode()`         | Switch game mode                      |
| `move_all(dx, dy)`   | Move all entities with Position       |
| `damage_all(amount)` | Damage all entities with Health       |
| `tick()`             | Advance the game by one turn          |
| `get_turn()`         | Get the current turn number           |
| `print_positions()`  | Print all entity positions            |
| `print_healths()`    | Print all entity healths              |

**Example Lua script (turn system):**

```lua
local id = spawn_entity()
set_component(id, "Position", { x = 0.0, y = 0.0 })
set_component(id, "Health", { current = 10.0, max = 10.0 })

print_positions()
print_healths()
print("Turn: " .. get_turn())

tick()
print_positions()
print_healths()
print("Turn: " .. get_turn())
```

**Example output:**

```text
Entity 1: Object {"x": Number(0), "y": Number(0)}
Entity 1: Object {"max": Number(10), "current": Number(10)}
Turn: 0
Entity 1: Object {"x": Number(1.0), "y": Number(0.0)}
Entity 1: Object {"max": Number(10), "current": Number(9.0)}
Turn: 1
```

To try out the new systems, run one of the Lua demo scripts:

```bash
cargo run --bin mge-cli -- engine/scripts/lua/turn_demo.lua
```

See the [Scripting (Lua)](#scripting-lua) section above for available functions and usage patterns.

**For engine developers:**
Lua scripts are run as part of the Rust integration tests to ensure scripting API stability and correctness:

```bash
cargo test -p engine_core
```

**For users/modders:**

**Adding new Lua-exposed ECS features:**

- Just define a new Rust component and register it with the ECS.
- No manual scripting bridge changes are needed.
- All components are accessible from Lua via `set_component(entity, "ComponentName", { ... })` and `get_component(entity, "ComponentName")`.
- Add Lua and Rust tests as needed.

> **Note**:
> The Lua scripting bridge is fully generic and mode-aware. Any registered ECS component can be set or queried from Lua using `set_component` and `get_component`, but only if it is valid for the current mode. No Rust-side scripting boilerplate is required for new components.

See [`engine/core/src/scripting/mod.rs`](engine/core/src/scripting/mod.rs) for details and documentation.

---

## CLI: Running Lua Scripts

You can run ECS-enabled Lua scripts directly from the command line using the `mge-cli` tool:

```bash
cargo run --bin mge-cli -- engine/scripts/lua/demo.lua
```

This executes your Lua script with full access to the ECS scripting API (`spawn_entity`, `set_component`, `get_component`, `set_mode`, etc.), including mode enforcement.
Any errors or output from the script will be shown in your terminal.

**Example output:**

```text
From file: pos.x=1.1 pos.y=2.2
```

See the [Scripting (Lua)](#scripting-lua) section above for available functions and usage patterns.

---

## Getting Started

1. **Explore the ECS Core:**

   - See [`engine/core`](engine/core) and [`engine_macros`](engine_macros).
   - Define components with `#[component(...)]` and auto-generate schemas.
   - Try switching modes at runtime and see how component access is enforced.

2. **Run tests:**

```bash
cargo test
```

3. **Read [`docs/idea.md`](docs/idea.md) for the full blueprint.**

---

## Related Crates

- [`engine/core`](engine/core): ECS core, registry, migration, and schema logic.
- [`engine_macros`](engine_macros): Procedural macros for ergonomic component definition.

---

## Continuous Integration & Releases

- **CI**: All pull requests and pushes to main run automated checks (formatting, linting, tests) via GitHub Actions.

- **Release**: Merges to main automatically trigger a semantic-release pipeline:

  - Versioning and changelog are managed by semantic-release.

  - Rust crate version is updated using `@timada/semantic-release-cargo`.

  - Releases are published to GitHub Releases

## Branch Protection:

The main branch is protected: all PRs require passing status checks before merging.
