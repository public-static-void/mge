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
- **Game systems like movement, health, turns, death, and decay are scriptable.**
- Lua scripts are loaded from `engine/scripts/lua/` and can be tested and run as part of the engine.
- You can switch game modes at runtime and only access components valid for the current mode.
- Attempting to set or get a component not available in the current mode will result in an error.

**Available Lua functions:**

| Function             | Description                                      |
| -------------------- | ------------------------------------------------ |
| `spawn_entity()`     | Spawn a new entity, returns entity id            |
| `set_component()`    | Set a component on an entity                     |
| `get_component()`    | Get a component from an entity                   |
| `set_mode()`         | Switch game mode                                 |
| `move_all(dx, dy)`   | Move all entities with Position                  |
| `damage_all(amount)` | Damage all entities with Health                  |
| `tick()`             | Advance the game by one turn                     |
| `get_turn()`         | Get the current turn number                      |
| `print_positions()`  | Print all entity positions                       |
| `print_healths()`    | Print all entity healths                         |
| `process_deaths()`   | Convert dead entities to corpses and start decay |
| `process_decay()`    | Decrement decay, remove entities when done       |
| `remove_entity(id)`  | Remove an entity and all its components          |

---

### Entity Death, Corpses, and Decay

MGE supports a robust entity lifecycle:

- When an entity’s `Health.current` drops to zero or below, you can call `process_deaths()` to convert it into a **corpse** (`Corpse` component) and give it a **decay timer** (`Decay` component).
- Each time you call `process_decay()`, the decay timer for all corpses is decremented. When it reaches zero, the entity is removed from the world.
- You can also remove entities directly with `remove_entity(id)`.

**Example Lua script:**

```lua
local id = spawn_entity()
set_component(id, "Health", { current = 2, max = 10 })
set_component(id, "Position", { x = 0, y = 0 })

-- Simulate death by setting health to zero
set_component(id, "Health", { current = 0, max = 10 })

process_deaths()
print("Corpse:", get_component(id, "Corpse"))
print("Decay:", get_component(id, "Decay"))

for i = 1, 5 do
    process_decay()
    print("Decay after tick " .. i .. ":", get_component(id, "Decay"))
end

-- After enough calls, get_component(id, "Corpse") and get_component(id, "Decay") will return nil
```

---

### Example: Turn System Script

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

---

## CLI: Running Lua Scripts

You can run any ECS-enabled Lua script using:

```bash
cargo run --bin mge-cli -- engine/scripts/lua/<script_name>.lua
```

For example, to run the turn demo:

```bash
cargo run --bin mge-cli -- engine/scripts/lua/turn_demo.lua
```

Or to test death/removal and decay:

```bash
cargo run --bin mge-cli -- engine/scripts/lua/death_removal_demo.lua
```

This executes your Lua script with full access to the ECS scripting API, including mode enforcement.
Any errors or output from the script will be shown in your terminal.

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

## Branch Protection

The main branch is protected: all PRs require passing status checks before merging.
