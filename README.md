# Modular Game Engine (MGE)

MGE is a modular, cross-language game engine blueprint and reference implementation for simulation, games, and rapid prototyping.

## Features

- Macro-driven ECS in Rust
- Hot-reloadable plugins & cross-language scripting (Lua, Python, WASM)
- Schema-driven, versioned, and mode-restricted components
- Runtime mode switching with enforcement in both Rust and scripting
- Lua scripting bridge for rapid prototyping
- Extensible architecture for tooling and modding

See [docs/idea.md](docs/idea.md) for a deeper dive.

---

## Quickstart

**Try the interactive roguelike demo:**

```bash
cargo run --bin mge-cli -- engine/scripts/lua/roguelike_mvp.lua
```

**Controls:**
`w/a/s/d` = move, `e` = attack, `q` = quit

The demo script (`engine/scripts/lua/roguelike_mvp.lua`) showcases MGE’s scripting, ECS, and runtime mode enforcement.

---

## Usage

- **Run any Lua script:**

  ```bash
  cargo run --bin mge-cli -- engine/scripts/lua/<script_name>.lua
  ```

- **Create a World in Rust:**

  ```rust
  let registry = Arc::new(ComponentRegistry::new());
  let mut world = World::new(registry.clone());
  ```

- **Add schemas:**
  Place JSON schemas in `engine/assets/schemas/` (see example below).

---

## Schema Example

```json
{
  "title": "Health",
  "type": "object",
  "properties": {
    "current": { "type": "number", "minimum": 0, "maximum": 100 },
    "max": { "type": "number", "minimum": 1, "maximum": 100 }
  },
  "required": ["current", "max"],
  "modes": ["colony", "roguelike"]
}
```

---

## Lua Scripting

- Spawn entities, set/get components, and interact with the ECS from Lua.
- Game systems like movement, health, turns, death, and decay are scriptable.
- Switch game modes at runtime; only access components valid for the current mode.
- See [docs/examples.md](docs/examples.md) for more.

---

## Limitations & Roadmap

- Rust components are not auto-registered; external schemas are loaded only in tests by default.
- Planned: runtime registration and schema loading, easier API for adding components/schemas.

---

## Resources

- [docs/idea.md](docs/idea.md): Architecture/design
- [docs/examples.md](docs/examples.md): Usage examples
- [docs/lua_api.md](docs/lua_api.md): Lua API reference
