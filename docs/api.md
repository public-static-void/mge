# MGE Scripting API Reference

> **Note:**
> All functions below are available in both **Lua** (as global functions) and **Python** (as methods on the `PyWorld` class).
> See [docs/examples.md](examples.md) for usage in both languages.

---

## Entity Management

| Function             | Description                             |
| -------------------- | --------------------------------------- |
| `spawn_entity()`     | Spawn a new entity                      |
| `despawn_entity(id)` | Remove an entity and all its components |
| `get_entities()`     | List all entity IDs                     |

---

## Component Access

| Function                                     | Description                                    |
| -------------------------------------------- | ---------------------------------------------- |
| `set_component(id, name, data)`              | Set a component on an entity                   |
| `get_component(id, name)`                    | Get a component from an entity                 |
| `remove_component(id, name)`                 | Remove a component from an entity              |
| `list_components()`                          | List all registered component names            |
| `get_component_schema(name)`                 | Get the JSON schema for a component            |
| `modify_stockpile_resource(id, kind, delta)` | Add/remove a resource in a Stockpile component |

---

## Entity Queries

| Function                              | Description                                |
| ------------------------------------- | ------------------------------------------ |
| `get_entities_with_component(name)`   | List entity ids with a given component     |
| `get_entities_with_components(names)` | List entity ids with all listed components |
| `is_entity_alive(id)`                 | Returns true if entity's Health > 0        |
| `count_entities_with_type(type_str)`  | Count entities with Type.kind == type_str  |

---

## Game Modes

| Function                | Description               |
| ----------------------- | ------------------------- |
| `set_mode(name)`        | Switch/set game mode      |
| `get_mode()`            | Get the current game mode |
| `get_available_modes()` | List all available modes  |

---

## Systems

| Function                    | Description                                |
| --------------------------- | ------------------------------------------ |
| `register_system(name, fn)` | Register a function as a named system      |
| `run_system(name)`          | Run a previously registered system by name |

---

## Event Bus

| Function                 | Description                                  |
| ------------------------ | -------------------------------------------- |
| `send_event(type, data)` | Send an event of a specific type             |
| `poll_event(type)`       | Poll all events of a type since last update  |
| `update_event_buses()`   | Advance all event buses (call once per tick) |

---

## Region and Zone Queries

| Function                              | Description                                            |
| -------------------------------------- | ------------------------------------------------------ |
| `get_entities_in_region(region_id)`    | List entity IDs assigned to a given region/zone        |
| `get_entities_in_region_kind(kind)`    | List entity IDs assigned to regions of the given kind  |
| `get_cells_in_region(region_id)`       | List cells assigned to a given region/zone             |
| `get_cells_in_region_kind(kind)`       | List cells assigned to regions of the given kind       |
---

## Worldgen Plugins

| Function                        | Description                              |
| ------------------------------- | ---------------------------------------- |
| `register_worldgen(name, fn)`   | Register a worldgen plugin               |
| `list_worldgen()`               | List all registered worldgen plugins     |
| `invoke_worldgen(name, params)` | Invoke a worldgen plugin with parameters |

---

## Persistence

| Function               | Description                            |
| ---------------------- | -------------------------------------- |
| `save_to_file(path)`   | Save the current world state to a file |
| `load_from_file(path)` | Load world state from a file           |

---

## Other Functions

| Function                  | Description                                 |
| ------------------------- | ------------------------------------------- |
| `move_entity(id, dx, dy)` | Move an entity by (dx, dy)                  |
| `move_all(dx, dy)`        | Move all entities with Position             |
| `damage_entity(id, amt)`  | Damage an entity (reduces Health)           |
| `damage_all(amt)`         | Damage all entities with Health             |
| `tick()`                  | Advance the game by one tick                |
| `get_turn()`              | Get the current turn number                 |
| `process_deaths()`        | Process deaths and start decay              |
| `process_decay()`         | Process decay timers and remove entities    |
| `get_user_input(prompt)`  | Prompt the user for input and return string |

---

## Notes

- All Lua functions are available globally in scripts.
- All Python functions are methods on the `PyWorld` class.
- Component names are case-sensitive and must match the `"title"` in your schema files.
- For practical code, see [docs/examples.md](examples.md).
