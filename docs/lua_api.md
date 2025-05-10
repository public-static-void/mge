# MGE Lua Scripting API Reference

This document lists the available Lua scripting functions provided by the Modular Game Engine (MGE).
Use these functions in your Lua scripts to interact with the ECS, control game flow, and implement gameplay logic.

---

## Entity Management

| Function            | Description                             |
| ------------------- | --------------------------------------- |
| `spawn_entity()`    | Spawn a new entity, returns entity id   |
| `remove_entity(id)` | Remove an entity and all its components |

---

## Component Access

| Function                        | Description                    |
| ------------------------------- | ------------------------------ |
| `set_component(id, name, data)` | Set a component on an entity   |
| `get_component(id, name)`       | Get a component from an entity |

---

## Game Modes

| Function         | Description      |
| ---------------- | ---------------- |
| `set_mode(name)` | Switch game mode |

---

## Gameplay Systems

| Function             | Description                     |
| -------------------- | ------------------------------- |
| `move_all(dx, dy)`   | Move all entities with Position |
| `damage_all(amount)` | Damage all entities with Health |
| `tick()`             | Advance the game by one turn    |
| `get_turn()`         | Get the current turn number     |

---

## Output and Debugging

| Function            | Description                |
| ------------------- | -------------------------- |
| `print_positions()` | Print all entity positions |
| `print_healths()`   | Print all entity healths   |

---

## Entity Lifecycle

| Function           | Description                                      |
| ------------------ | ------------------------------------------------ |
| `process_deaths()` | Convert dead entities to corpses and start decay |
| `process_decay()`  | Decrement decay, remove entities when done       |

---

## User Input

| Function           | Description                                   |
| ------------------ | --------------------------------------------- |
| `get_user_input()` | Prompt the user for input and return a string |

---

## Notes

- All functions are available globally in MGE Lua scripts.
- Component names are case-sensitive and must match the `"title"` field in a registered schema (see `engine/assets/schemas/`).
- Only components allowed in the current mode (as defined in their schema's `"modes"` array) can be accessed or set.
- You can add new components for scripting by simply adding new schema files-no Rust code required.
- For more examples, see [docs/examples.md](examples.md).
