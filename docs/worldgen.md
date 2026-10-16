# Worldgen Plugin System

MGE supports a flexible, multi-language world generation plugin system. Plugins can be written in Rust, Lua, Python, or C (via ABI), registered at runtime, listed, and invoked from any supported language.

---

## Registering Plugins

### Rust

```rust
registry.register(WorldgenPlugin::CAbi {
    name: "simple_square".to_string(),
    generate: Box::new(|params| { /* generate map JSON */ }),
});
```

### Lua

```lua
register_worldgen_plugin("luagen", function(params)
    -- params is a Lua table representing JSON
    return { cells = { { id = "luacell", x = 1, y = 2 } } }
end)
```

### Python

```python
def pygen(params):
    return {"cells": [{"id": "pycell", "x": 0, "y": 0}]}

world.register_worldgen_plugin("pygen", pygen)
```

### C ABI

See [`engine/engine_plugin_abi.h`](../engine/engine_plugin_abi.h) for details on creating C plugins.

---

## Listing Plugins

- Lua: `list_worldgen_plugins()`
- Python: `world.list_worldgen_plugins()`
- Rust: `registry.list_names()`

---

## Invoking Plugins

- Lua:

```lua
local result = invoke_worldgen_plugin("luagen", { width = 7 })
```

- Python:

```python
result = world.invoke_worldgen_plugin("pygen", {"width": 7})
```

- Rust:

```rust
let params = serde_json::json!({ "width": 10 });
let result = registry.invoke("simple_square", &params)?;
```

---

## Builtin algorithms

Three mapgen algorithms ship in-core and resolve by name string through `invoke_worldgen_plugin` (Lua global, `world.invoke_worldgen_plugin` in Python, WASM host function — WASM params travel as a JSON string of the same shape). They are registered on both the thread-safe (global/WASM/Python) and local (Lua) registries by `register_builtin_mapgen_algorithms` / `register_builtin_mapgen_algorithms_local`.

### Algorithm selection

`resolve_algorithm_name` picks the active algorithm: the `algorithm` field in the invocation params wins over the world-config default, which wins over the `dungeon` fallback (`FALLBACK_MAPGEN_ALGORITHM = "dungeon"`). Switching algorithms is a name-string change — no source edit.

| Invocation name | Description | Parameters (defaults) | Output shape |
| --- | --- | --- | --- |
| `dungeon` | Rooms plus L-shaped corridors. | `width` (40), `height` (25), `seed` (time-seeded; an explicit `seed` always wins), `min_room_size` (3), `max_room_size` (8), `max_rooms` (10). Zero dimensions fail. | `{topology: "square", cells: [{x, y, z, neighbors, metadata}]}`. Wall cells carry `{walkable: false, transparent: false}` metadata so the pathfinder treats them as impassable; floor cells carry no metadata. |
| `caves` | Cellular-automata caverns (random fill plus smoothing passes). | `width` (40), `height` (25), `seed` (time-seeded; an explicit `seed` always wins), `fill_chance` (0.45), `steps` (4), `birth_limit` (5), `death_limit` (4). Minimum 3x3; zero dimensions fail. | Same square worldgen JSON as `dungeon` (the walkable floor set converts through the shared dungeon-map shape). |
| `planet_biomes` | Planet-scale biome field. *Lands with the gen0 planet-biome provider merge — not available on builds predating that merge.* | `width`, `height`, `seed`; defaults 32x32; `width * height` over the 1M-cell cap (`MAX_CELLS = 1_000_000`) fails fast. Missing/null `seed` falls back to `time_seed()`; an explicit seed always wins; unknown keys are ignored. | `square` topology; per-cell `x`/`y`/`z = 0`, `biome` label, full 4-neighbourhood. Mountain cells carry a mountain-only `terrain` slot mirrored into `metadata.terrain` (`walkable`/`transparent` unset). |

### Registry path only

The registry path — `list_worldgen_plugins`, `invoke_worldgen_plugin`, `register_worldgen_plugin`, the validator/postprocessor registration hooks, plus `apply_generated_map` — is the only supported way to reach the builtins above. The legacy per-bridge `generate_dungeon` direct helpers (`engine_lua/src/lua_api/dungeon.rs`, `engine_py/src/python_api/dungeon.rs`, `engine_wasm/src/host_api/dungeon.rs`) predate the registry and sit outside this claim. No `generate_planet_biomes` helper shall be added — invoke `planet_biomes` via `invoke_worldgen_plugin` by name string.

---

## Data Interchange

- Parameters and results are passed as JSON-like structures (`serde_json::Value` in Rust, tables in Lua, dicts in Python).
- Conversion between Lua tables and JSON is automatic.
- Errors in invocation or missing plugins produce clear error messages.

---

## Examples

See [`engine/scripts/lua/tests/test_worldgen.lua`](../engine/scripts/lua/tests/test_worldgen.lua) and [`engine_py/tests/test_worldgen.py`](../engine_py/tests/test_worldgen.py) for working examples.

---

## Notes

- Lua plugins store the Lua function in the registry and are invoked with automatic marshaling.
- Python plugins are standard Python callables.
- C ABI plugins follow a stable `vtable` interface.
- The system supports runtime registration and invocation, enabling dynamic world generation from any supported language.
