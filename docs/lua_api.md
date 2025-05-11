# MGE Lua Scripting API Reference

This document lists the available Lua scripting functions provided by the Modular Game Engine (MGE).
Use these functions in your Lua scripts to interact with the ECS, control game flow, and implement gameplay logic.

---

## Entity Management

| Function            | Description                             |
| ------------------- | --------------------------------------- |
| `spawn_entity()`    | Spawn a new entity, returns entity id   |
| `remove_entity(id)` | Remove an entity and all its components |
| `get_entities()`    | List all entity IDs                     |

---

## Component Access

| Function                                     | Description                                                                                                               |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `set_component(id, name, data)`              | Set a component on an entity                                                                                              |
| `get_component(id, name)`                    | Get a component from an entity                                                                                            |
| `remove_component(id, name)`                 | Remove a component from an entity                                                                                         |
| `list_components()`                          | List all registered component names                                                                                       |
| `get_component_schema(name)`                 | Get the JSON schema for a component                                                                                       |
| `modify_stockpile_resource(id, kind, delta)` | Add or remove (delta can be negative) a specific resource in a Stockpile component. Returns error if not enough resource. |

---

## Entity Queries

| Function                              | Description                                        |
| ------------------------------------- | -------------------------------------------------- |
| `get_entities_with_component(name)`   | List entity ids with a given component             |
| `get_entities_with_components(names)` | List entity ids with all listed components (table) |
| `is_entity_alive(id)`                 | Returns true if entity's Health > 0                |
| `count_entities_with_type(type_str)`  | Count entities with Type.kind == type_str          |

---

## Game Modes

| Function                | Description               |
| ----------------------- | ------------------------- |
| `set_mode(name)`        | Switch game mode          |
| `get_mode()`            | Get the current game mode |
| `get_available_modes()` | List all available modes  |

---

## Gameplay Systems

| Function                    | Description                                      |
| --------------------------- | ------------------------------------------------ |
| `move_entity(id, dx, dy)`   | Move an entity by (dx, dy)                       |
| `move_all(dx, dy)`          | Move all entities with Position                  |
| `damage_entity(id, amount)` | Damage an entity (reduces Health)                |
| `damage_all(amount)`        | Damage all entities with Health                  |
| `tick()`                    | Advance the game by one turn                     |
| `get_turn()`                | Get the current turn number                      |
| `process_deaths()`          | Convert dead entities to corpses and start decay |
| `process_decay()`           | Decrement decay, remove entities when done       |

---

## Output and Debugging

| Function            | Description                |
| ------------------- | -------------------------- |
| `print_positions()` | Print all entity positions |
| `print_healths()`   | Print all entity healths   |

---

## User Input

| Function                 | Description                                   |
| ------------------------ | --------------------------------------------- |
| `get_user_input(prompt)` | Prompt the user for input and return a string |

---

## Notes

- All functions are available globally in MGE Lua scripts.
- Component names are case-sensitive and must match the `"title"` field in a registered schema (see `engine/assets/schemas/`).
- Only components allowed in the current mode (as defined in their schema's `"modes"` array) can be accessed or set.
- You can add new components for scripting by simply adding new schema files-no Rust code required.
- For more examples, see [docs/examples.md](examples.md).
