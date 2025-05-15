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
register_worldgen("luagen", function(params)
    -- params is a Lua table representing JSON
    return { cells = { { id = "luacell", x = 1, y = 2 } } }
end)
```

### Python

```python
def pygen(params):
    return {"cells": [{"id": "pycell", "x": 0, "y": 0}]}

world.register_worldgen("pygen", pygen)
```

### C ABI

See [`engine/engine_plugin_abi.h`](../engine/engine_plugin_abi.h) for details on creating C plugins.

---

## Listing Plugins

- Lua: `list_worldgen()`
- Python: `world.list_worldgen()`
- Rust: `registry.list_names()`

---

## Invoking Plugins

- Lua:

```lua
local result = invoke_worldgen("luagen", { width = 7 })
```

- Python:

```python
result = world.invoke_worldgen("pygen", {"width": 7})
```

- Rust:

```rust
let params = serde_json::json!({ "width": 10 });
let result = registry.invoke("simple_square", &params)?;
```

---

## Data Interchange

- Parameters and results are passed as JSON-like structures (`serde_json::Value` in Rust, tables in Lua, dicts in Python).
- Conversion between Lua tables and JSON is automatic.
- Errors in invocation or missing plugins produce clear error messages.

---

## Examples

See [`engine/scripts/lua/test_worldgen.lua`](../engine/scripts/lua/test_worldgen.lua) and [`engine_py/tests/test_worldgen.py`](../engine_py/tests/test_worldgen.py) for working examples.

---

## Notes

- Lua plugins store the Lua function in the registry and are invoked with automatic marshaling.
- Python plugins are standard Python callables.
- C ABI plugins follow a stable `vtable` interface.
- The system supports runtime registration and invocation, enabling dynamic world generation from any supported language.
