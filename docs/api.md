# MGE Scripting API Reference

> **Note:**
> All functions below are available in both **Lua** (as global functions or methods on the `world` userdata) and **Python** (as methods on the `PyWorld` class).
> See [docs/examples.md](examples.md) for usage in both languages.

---

## Entity Management

| Function             | Description                             |
| -------------------- | --------------------------------------- |
| `despawn_entity(id)` | Remove an entity and all its components |
| `spawn_entity()`     | Spawn a new entity                      |

---

## Component Management

| Function                                     | Description                                    |
| -------------------------------------------- | ---------------------------------------------- |
| `get_component(id, name)`                    | Get a component from an entity                 |
| `get_component_schema(name)`                 | Get the JSON schema for a component            |
| `list_components()`                          | List all registered component names            |
| `modify_stockpile_resource(id, kind, delta)` | Add/remove a resource in a Stockpile component |
| `remove_component(id, name)`                 | Remove a component from an entity              |
| `set_component(id, name, data)`              | Set a component on an entity                   |

---

## Entity Queries

| Function                              | Description                                           |
| ------------------------------------- | ----------------------------------------------------- |
| `count_entities_with_type(type_str)`  | Count entities with Type.kind == type_str             |
| `get_entities()`                      | List all entity IDs                                   |
| `get_entities_in_region(region_id)`   | List entity IDs assigned to a given region/zone       |
| `get_entities_in_region_kind(kind)`   | List entity IDs assigned to regions of the given kind |
| `get_entities_with_component(name)`   | List entity ids with a given component                |
| `get_entities_with_components(names)` | List entity ids with all listed components            |
| `is_entity_alive(id)`                 | Returns true if entity's Health > 0                   |

---

## Region and Zone Queries

| Function                         | Description                                      |
| -------------------------------- | ------------------------------------------------ |
| `get_cells_in_region(region_id)` | List cells assigned to a given region/zone       |
| `get_cells_in_region_kind(kind)` | List cells assigned to regions of the given kind |

---

## Map and Topology

| Function                           | Description                                    |
| ---------------------------------- | ---------------------------------------------- |
| `add_cell(x, y, z)`                | Add a cell to the map at coordinates (x, y, z) |
| `add_neighbor(from, to)`           | Add a neighbor relationship between two cells  |
| `entities_in_cell(cell)`           | List all entity IDs in the given cell          |
| `find_path(start_cell, goal_cell)` | Find a path between two cells                  |
| `get_all_cells()`                  | List all cells in the current map              |
| `get_map_cell_count()`             | Get the number of cells in the map             |
| `get_map_topology_type()`          | Get the topology type of the map               |
| `get_neighbors(cell)`              | List neighbors of a given cell                 |

---

## Map/Cell Metadata

| Function                            | Description              |
| ----------------------------------- | ------------------------ |
| `get_cell_metadata(cell)`           | Get metadata for a cell. |
| `set_cell_metadata(cell, metadata)` | Set metadata for a cell. |

---

## Map Generation, Validation, and Postprocessor Hooks

> **Note:**
> In **Lua**, methods on the `world` object use a colon (`:`):
>
> ```lua
> world:register_map_validator(function(map) ... end)
> world:register_map_postprocessor(function(w) ... end)
> ```
>
> In **Python**, use a dot (`.`):
>
> ```python
> world.register_map_validator(validator)
> world.register_map_postprocessor(postprocessor)
> ```
>
> Validators run **before** the map is applied, and can block invalid maps.
> Postprocessors run **after** the map is applied, and can mutate the world.

| Function                           | Description                                                                                                                             |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `apply_generated_map(map)`         | Apply a generated map (as a table/dict). Runs all registered validators first, then applies the map, then postprocessors.               |
| `register_map_validator(func)`     | Register a function to be called before every map application. Receives the map as argument. If any returns false, the map is rejected. |
| `clear_map_validators()`           | Clear all registered map validator functions.                                                                                           |
| `register_map_postprocessor(func)` | Register a function to be called after every map application. Receives the world object as argument. Errors abort application.          |
| `clear_map_postprocessors()`       | Clear all registered map postprocessor functions.                                                                                       |

---

## Worldgen Plugins

| Function                               | Description                              |
| -------------------------------------- | ---------------------------------------- |
| `invoke_worldgen_plugin(name, params)` | Invoke a worldgen plugin with parameters |
| `list_worldgen_plugins()`              | List all registered worldgen plugins     |
| `register_worldgen_plugin(name, fn)`   | Register a worldgen plugin               |

---

## Job System

| Function (Lua) / Method (Python)                                                                   | Description                                                                        |
| -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `assign_job(entity, job_type, fields?)`<br>`world.assign_job(entity, job_type, fields=None)`       | Assign a job of the given type to an entity. Optional fields/dict sets properties. |
| `cancel_job(job_id)`<br>`world.cancel_job(job_id)`                                                 | Cancel a job by unique ID.                                                         |
| `set_job_field(job_id, field, value)`<br>`world.set_job_field(job_id, field, value)`               | Set a single field (by name) on a job.                                             |
| `update_job(job_id, fields)`<br>`world.update_job(job_id, fields)`                                 | Update multiple fields on a job (fields table/dict).                               |
| `list_jobs([opts])`<br>`world.list_jobs(opts=None)`                                                | List jobs. Optionally filter with a table/dict.                                    |
| `get_job(job_id)`<br>`world.get_job(job_id)`                                                       | Retrieve a job by its ID.                                                          |
| `find_jobs([filter])`<br>`world.find_jobs(filter=None)`                                            | Find jobs matching a filter table/dict.                                            |
| `advance_job_state(job_id)`<br>`world.advance_job_state(job_id)`                                   | Advance a job’s internal state machine.                                            |
| `get_job_children(job_id)`<br>`world.get_job_children(job_id)`                                     | Get list of child job IDs for a job.                                               |
| `set_job_children(job_id, children)`<br>`world.set_job_children(job_id, children)`                 | Set list of child job IDs for a job.                                               |
| `get_job_dependencies(job_id)`<br>`world.get_job_dependencies(job_id)`                             | Get job IDs this job depends on.                                                   |
| `set_job_dependencies(job_id, deps)`<br>`world.set_job_dependencies(job_id, deps)`                 | Set dependency job IDs for a job.                                                  |
| `get_job_board()`<br>`world.get_job_board()`                                                       | Return the current job scheduling board.                                           |
| `get_job_board_policy()`<br>`world.get_job_board_policy()`                                         | Get the current job scheduling policy name.                                        |
| `set_job_board_policy(policy)`<br>`world.set_job_board_policy(policy)`                             | Set the job scheduling policy by name.                                             |
| `get_job_priority(job_id)`<br>`world.get_job_priority(job_id)`                                     | Get the priority value of a job.                                                   |
| `set_job_priority(job_id, value)`<br>`world.set_job_priority(job_id, value)`                       | Set the priority value of a job.                                                   |
| `add_job_to_job_board(job_id)` _(Lua only)_                                                        | Add a job to the job scheduling board.                                             |
| `get_job_types()`<br>`world.get_job_types()`                                                       | List all registered job types.                                                     |
| `register_job_type(name, fn/callback)`<br>`world.register_job_type(name, callback)`                | Register a custom job type with a script-side handler/callback.                    |
| `get_job_type_metadata(name)`<br>`world.get_job_type_metadata(name)`                               | Get metadata/schema for a job type.                                                |
| `ai_assign_jobs(agent_id, args?)`<br>`world.ai_assign_jobs(agent_id, args=None)`                   | Assign jobs to an AI agent, optionally with args.                                  |
| `ai_query_jobs(agent_id)`<br>`world.ai_query_jobs(agent_id)`                                       | Query which jobs are available to an AI agent.                                     |
| `ai_modify_job_assignment(job_id, changes)`<br>`world.ai_modify_job_assignment(job_id, changes)`   | Change job assignment for agent/job.                                               |
| `get_production_job(entity)`<br>`world.get_production_job(entity)`                                 | Get ProductionJob component for an entity.                                         |
| `get_production_job_progress(entity)`<br>`world.get_production_job_progress(entity)`               | Get progress of a ProductionJob for the entity.                                    |
| `set_production_job_progress(entity, value)`<br>`world.set_production_job_progress(entity, value)` | Set progress of a ProductionJob.                                                   |
| `get_production_job_state(entity)`<br>`world.get_production_job_state(entity)`                     | Get current state/status of ProductionJob.                                         |
| `set_production_job_state(entity, value)`<br>`world.set_production_job_state(entity, value)`       | Set state/status on ProductionJob.                                                 |
| `get_job_resource_reservations(entity)`<br>`world.get_job_resource_reservations(entity)`           | Get resource reservations for an entity's job.                                     |
| `reserve_job_resources(entity)`<br>`world.reserve_job_resources(entity)`                           | Reserve resources for the entity’s job(s).                                         |
| `release_job_resource_reservations(entity)`<br>`world.release_job_resource_reservations(entity)`   | Release reserved resources for jobs.                                               |

---

### Job Event Log API

> _Lua:_ All functions below are methods on the `job_events` table.
> _Python:_ All functions are methods on `PyWorld` (e.g. `world.get_job_event_log()`).

| Function / Method                                                                               | Description                                                            |
| ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `job_events.get_all_events()`<br>`get_job_event_log()`                                          | Get all recorded job events from the job event log.                    |
| `job_events.get_events_by_type(event_type)`<br>`get_job_events_by_type(event_type)`             | Get job events filtered by event type.                                 |
| `job_events.get_events_by_timestamp(ts)`<br>`get_job_events_since(ts)`                          | Get job events with timestamp ≥ supplied value.                        |
| `job_events.get_events_matching_type(type)`<br>`get_job_events_where(predicate)`                | (Lua: filter by type substring. Python: filter by predicate function.) |
| `job_events.subscribe(event_type, callback)`<br>`subscribe_job_event_bus(event_type, callback)` | Register a callback for job events of the given type.                  |
| `job_events.unsubscribe(event_type, sub_id)`<br>`unsubscribe_job_event_bus(event_type, sub_id)` | Remove a previously registered event callback subscription.            |
| `job_events.get_last_events()` _(Lua only)_                                                     | Get the most recent batch of job events.                               |
| `job_events.save_event_log(path)`<br>`save_job_event_log(path)`                                 | Save current job event log to a file.                                  |
| `job_events.load_event_log(path)`<br>`load_job_event_log(path)`                                 | Load job event log from file.                                          |
| `job_events.replay_event_log()`<br>`replay_job_event_log()`                                     | Replay loaded event log, applying events to the simulation.            |
| `job_events.clear_all_events()`<br>`clear_job_event_log()`                                      | Clear all stored job events from the log.                              |

---

### Utility (Python)

| Function                     | Description                                                                 |
| ---------------------------- | --------------------------------------------------------------------------- |
| `py_init_job_event_logger()` | Initialize the job event logger system. Typically called once during setup. |

---

## Inventory, Equipment, and Body Management

| Function                                    | Description                                 |
| ------------------------------------------- | ------------------------------------------- |
| `add_body_part(entity, part)`               | Add a part to the body (recursive)          |
| `add_item_to_inventory(entity, item_id)`    | Add an item ID to the inventory             |
| `equip_item(entity, item_id, slot)`         | Equip an item to a slot                     |
| `get_body(entity)`                          | Get the body component as a table/dict      |
| `get_body_part(entity, name)`               | Get a part by name (recursive)              |
| `get_equipment(entity)`                     | Get the equipment component as a table/dict |
| `get_inventory(entity)`                     | Get the inventory component as a table/dict |
| `remove_body_part(entity, name)`            | Remove a part by name (recursive)           |
| `remove_item_from_inventory(entity, index)` | Remove item at index from the inventory     |
| `set_body(entity, data)`                    | Set the body component                      |
| `set_inventory(entity, data)`               | Set the inventory component                 |
| `unequip_item(entity, slot)`                | Unequip the item from the given slot        |

---

## Economic System

| Function                          | Description                                                |
| --------------------------------- | ---------------------------------------------------------- |
| `get_production_job(entity)`      | Get the full ProductionJob component as a table/dict       |
| `get_stockpile_resources(entity)` | Get the resources subtable/dict from a Stockpile component |

---

## Camera & Viewport

| Function           | Description                               |
| ------------------ | ----------------------------------------- |
| `get_camera()`     | Get the camera position as {x, y}         |
| `set_camera(x, y)` | Set the camera position (center viewport) |

---

## Time and Turn

| Function            | Description                                   |
| ------------------- | --------------------------------------------- |
| `get_time_of_day()` | Get the current time of day in the simulation |
| `get_turn()`        | Get the current turn number                   |
| `tick()`            | Advance the game by one tick                  |

---

## Systems

| Function                    | Description                                |
| --------------------------- | ------------------------------------------ |
| `poll_ecs_event(type)`      | Poll ECS events of a type                  |
| `register_system(name, fn)` | Register a function as a named system      |
| `run_native_system(name)`   | Run a built-in system by name              |
| `run_system(name)`          | Run a previously registered system by name |

---

## Event Bus

| Function                 | Description                                  |
| ------------------------ | -------------------------------------------- |
| `poll_event(type)`       | Poll all events of a type since last update  |
| `send_event(type, data)` | Send an event of a specific type             |
| `update_event_buses()`   | Advance all event buses (call once per tick) |

---

## Persistence

| Function               | Description                            |
| ---------------------- | -------------------------------------- |
| `load_from_file(path)` | Load world state from a file           |
| `save_to_file(path)`   | Save the current world state to a file |

---

## UI API

| Function                               | Description                                                      |
| -------------------------------------- | ---------------------------------------------------------------- |
| `ui.add_child(parent_id, child_id)`    | Add a widget as a child to another                               |
| `ui.create_widget(type, props)`        | Create a new UI widget of given type and properties              |
| `ui.focus_widget(id)`                  | Set focus to a widget                                            |
| `ui.get_children(parent_id)`           | Get the children IDs of a widget                                 |
| `ui.get_parent(id)`                    | Get the parent ID of a widget                                    |
| `ui.get_widget_props(id)`              | Get properties of a widget                                       |
| `ui.get_widget_type(id)`               | Get the type name of a widget (including dynamically registered) |
| `ui.get_z_order(id)`                   | Get the z-order of a widget                                      |
| `ui.load_json(json_str)`               | Load a UI widget tree from JSON                                  |
| `ui.register_widget(type, ctor)`       | Register a new widget type dynamically from scripting            |
| `ui.remove_callback(id, event)`        | Remove a callback for a widget event                             |
| `ui.remove_child(parent_id, child_id)` | Remove a child widget from a parent                              |
| `ui.remove_widget(id)`                 | Remove a widget by ID                                            |
| `ui.set_callback(id, event, fn)`       | Register a callback for a widget event (e.g., "click")           |
| `ui.set_widget_props(id, props)`       | Set properties on a widget                                       |
| `ui.set_z_order(id, z)`                | Set the z-order of a widget                                      |
| `ui.trigger_event(id, event, args)`    | Trigger an event on a widget                                     |

### Supported Widget Types

| Widget Type     | Properties (main)                                                   |
| --------------- | ------------------------------------------------------------------- |
| `"Button"`      | `label: string`, `pos: [x, y]`, `color: [r, g, b]`, `group?: int`   |
| `"Label"`       | `text: string`, `pos: [x, y]`, `color: [r, g, b]`                   |
| `"Checkbox"`    | `label: string`, `pos: [x, y]`, `checked: bool`, `color: [r, g, b]` |
| `"Dropdown"`    | `items: [string]`, `pos: [x, y]`, `selected: int`, `color: [r,g,b]` |
| `"TextInput"`   | `text: string`, `pos: [x, y]`, `color: [r, g, b]`                   |
| `"Panel"`       | `pos: [x, y]`, `size: [w, h]`, `color: [r, g, b]`                   |
| `"GridLayout"`  | `pos: [x, y]`, `rows: int`, `cols: int`, `spacing: int`             |
| `"ContextMenu"` | `items: [string]`, `pos: [x, y]`, `color: [r, g, b]`                |

#### Notes:

- All functions are available as `ui.<function>` in Lua, and as methods on `UiApi` in Python.
- Widget property tables/dicts use the scripting language’s native data structures.
- Dynamic widget registration allows to add new types at runtime from scripts.

---

## User Input

| Function                 | Description                                 |
| ------------------------ | ------------------------------------------- |
| `get_user_input(prompt)` | Prompt the user for input and return string |

---

## Global Objects

| Name        | Description                                                      |
| ----------- | ---------------------------------------------------------------- |
| `world`     | The world object for scripting. All methods above are available. |
| `ui`        | The UI API object. See UI API section for available methods.     |
| `event_bus` | The event bus API object. See Event Bus section for methods.     |

---

## Notes

- All Lua functions are available globally in scripts, or as methods on the `world` userdata.
- All Python functions are methods on the `PyWorld` class.
- Component names are case-sensitive and must match the `"title"` in your schema files.
- For practical code, see [docs/examples.md](examples.md).
