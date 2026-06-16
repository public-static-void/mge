# Modular Game Engine (MGE)

MGE is a modular, cross-language game engine blueprint for rapid prototyping, extensible games, and simulation.

---

## Capabilities

- **Schema-driven ECS:** JSON-based, hot-reloadable components and systems.
- **Unified scripting API:** Identical ECS/world APIs in Lua and Python.
- **Hot-reloadable plugins:** Extend at runtime with Rust, Lua, Python, or C.
- **Flexible world generation:** Register/invoke worldgen plugins in any supported language.
- **Dependency-aware system scheduler:** Automatic ordering and cycle detection.
- **Runtime mode switching:** Switch between game modes on the fly.
- **Deterministic simulation:** Modular, event-driven tick loop.
- **Region/zone queries:** Query entities/cells by region or kind.

---

## Usage

### Lua Scripting Example:

```sh
cargo run --bin mge_cli -- engine/scripts/lua/demos/roguelike_mvp.lua
```

A fully playable roguelike demo showcasing 8+ engine subsystems:

| Subsystem       | What it demonstrates                                         |
| --------------- | ------------------------------------------------------------ |
| Grid map        | 40×25 tile map with rooms, corridors, walls                  |
| Pathfinding     | AI enemies navigate using `find_path()`                      |
| Camera          | Viewport follows player                                      |
| ECS + Schemas   | Schema-defined Health, Position, Type, Renderable components |
| Inventory       | Pickup, use, drop items with visible inventory screen        |
| Simulation tick | Structured `tick()` + `get_turn()` game loop                 |
| Event bus       | Combat/death events displayed as message log                 |
| Death/decay     | Corpses, decay timer, loot drops on kill                     |
| Save/load       | 4 save slots with menu-driven save/load                      |

Controls: `WASD/hjkl` move · `.` wait · `e/g` pickup · `q/u` use · `i` inventory · `d` drop · `S` save · `L` load · `Q` quit

### Terminal Viewport Rendering Example:

MGE supports modular rendering backends. You can render a map and entities directly to your terminal for rapid prototyping and roguelike-style games.

Run the demo:

```bash
cargo run --example viewport_demo -p engine_core
```

Example output:

```bash
..........
..........
....@.....
..........
..........
```

### Roguelike MVP Viewport Demo Mod

```bash
cargo run --bin mge_cli -- --mod mvp_roguelike
```

Controls: `WASD` = move, `e` = attack, `.` = wait, `q` = quit

---

## Documentation

- [docs/dev.md](docs/dev.md): Developer setup & test guide
- [docs/idea.md](docs/idea.md): Architecture and design
- [docs/api.md](docs/api.md): Unified scripting API (Lua & Python)
- [docs/plugin_abi.md](docs/plugin_abi.md): C ABI plugin authoring
- [docs/examples.md](docs/examples.md): Usage examples (Lua, Python, Rust, C)
- [docs/worldgen.md](docs/worldgen.md): Worldgen plugin system
- [docs/ROADMAP.md](docs/ROADMAP.md): Project Roadmap
