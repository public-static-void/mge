# MGE C Plugin ABI Reference

This document describes the **C ABI** for writing hot-reloadable plugins for the Modular Game Engine (MGE).
Plugins can register systems, participate in world generation, and interact with the engine at runtime.

---

## Overview

- Plugins are compiled shared libraries (`.so`, `.dll`, `.dylib`) that export a single vtable symbol: `PLUGIN_VTABLE`.
- The vtable exposes function pointers for initialization, update, shutdown, world generation, and system registration.
- The ABI is defined in [`engine/engine_plugin_abi.h`](../engine/engine_plugin_abi.h).
- Plugins are hot-reloaded and run in the same process as the engine.

---

## VTable Structure

```c
typedef struct PluginVTable {
  int (*init)(struct EngineApi *api, void *world);
  void (*shutdown)();
  void (*update)(float delta_time);
  const char *(*worldgen_name)();
  int (*generate_world)(const char *params_json, char **out_result_json);
  void (*free_result_json)(char *result_json);
  int (*register_systems)(struct EngineApi *api, void *world,
                          SystemPlugin **systems, int *count);
  void (*free_systems)(SystemPlugin *systems, int count);
} PluginVTable;
```

- **init:** Called once after loading. Set up plugin state, register components, spawn entities, etc.
- **shutdown:** Called before unloading. Clean up resources.
- **update:** Called every frame/tick (if used).
- **worldgen_name:** Return the name of the worldgen plugin (for world generation plugins).
- **generate_world:** Generate a world based on JSON parameters. Returns a JSON string (see below).
- **free_result_json:** Free memory allocated for result JSON (if needed).
- **register_systems:** Register one or more ECS systems with the engine.
- **free_systems:**
  If you dynamically allocate the `SystemPlugin` array (e.g., with `malloc`), provide a function here to free it.
  If you use a static/global array, set this to `NULL`.

---

## Engine API

Plugins receive an `EngineApi` struct for interacting with the engine:

```c
typedef struct EngineApi {
  unsigned int (*spawn_entity)(void *world);
  int (*set_component)(void *world, unsigned int entity, const char *name,
                       const char *json_value);
} EngineApi;
```

- **spawn_entity:** Creates a new entity in the world. Returns the entity ID.
- **set_component:** Sets a component on an entity using a JSON string.

---

## System Registration ABI

To register ECS systems at runtime:

```c
typedef void (*SystemRunFn)(void *world, float delta_time);

typedef struct SystemPlugin {
  const char *name;
  SystemRunFn run;
} SystemPlugin;

// Example system
void hello_system(void *world, float delta_time) {
  // Implement your system logic here
}

// Static array of systems
static SystemPlugin system_plugins[] = {
  { "hello_system", hello_system }
};

// Register systems function
int register_systems(struct EngineApi *api, void *world, SystemPlugin **systems, int *count) {
  *systems = system_plugins;
  *count = 1;
  return 0;
}
```

- The array of `SystemPlugin` must be static/global (not stack-allocated).
- The `name` field must point to a static string for the plugin's lifetime.
- Return 0 on success.

- **Memory Management:**
  If you allocate the `SystemPlugin` array dynamically, you must provide a `free_systems` function in your vtable that will be called by the engine after registration.
  For static/global arrays, set `free_systems` to `NULL`.

---

## World Generation ABI

For worldgen plugins, implement:

```c
const char *worldgen_name(void);  // Returns the name of this generator

int generate_world(const char *params_json, char **out_result_json);
// params_json: JSON string with generation parameters
// out_result_json: set to a malloc'd string containing the generated world as JSON
// Return 0 on success, nonzero on failure

void free_result_json(char *result_json);
// Free memory allocated for result JSON
```

- The engine will call `worldgen_name` to list available generators.
- The engine will call `generate_world` and expects a JSON string describing the generated world.
- If you allocate memory for the result, provide a `free_result_json` function.

---

## Best Practices & Notes

- **Multiple Systems:**
  Register multiple systems by adding more entries to the static `SystemPlugin` array and setting `*count` accordingly in `register_systems`.

- **Memory Management:**
  If you allocate the `SystemPlugin` array dynamically (e.g., with `malloc`), you must ensure its memory remains valid for the plugin's lifetime. Static/global arrays are recommended.

- **Struct Layout:**
  All struct layouts must match exactly between C and Rust (`#[repr(C)]` in Rust).

- **Other Languages:**
  The C ABI is the foundation for all plugin types. Integrations for Lua, Python, etc., should use this ABI for maximum compatibility.

- **Unused VTable Fields:**
  Set unused function pointers in the vtable to `NULL`.

---

## Example

A minimal C plugin is shown in [docs/examples.md](examples.md#c-abi-plugin-example).

---

## See Also

- [`engine/engine_plugin_abi.h`](../engine/engine_plugin_abi.h) - C ABI header
- [docs/examples.md](examples.md) - Example plugins and usage
