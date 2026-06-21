# MGE Development Process

## Genre-Driven Development

MGE development is driven by targeting specific game genres. Each genre exposes what engine capabilities are needed — what systems, data structures, scripting APIs, and presentation features a real game requires. This prevents building abstract features that never get used in an actual game, keeps scope grounded, and provides immediate feedback on whether the architecture is serving real needs.

The development cycle follows: a target genre is chosen, required engine features are identified, the ROADMAP is consulted for what already exists, missing capabilities are implemented, and a demo or prototype validates that the new capabilities work together to produce something game-like. Lessons from each cycle — what was awkward, what was missing, what was over-engineered — feed back into the next iteration.

If a capability cannot be justified by a specific game need, it does not belong in the engine yet.

## Relationship to the ROADMAP

The ROADMAP is the feature inventory. It grows and gets refined as new game genres are targeted. Not all items need to be done for any single game — the ROADMAP reflects the total set of capabilities accumulated across all genre cycles, not a mandatory checklist for any one project.

## Current Example: Roguelike

The roguelike genre has driven the current implementation. This cycle produced:

- Grid maps with pathfinding (square, hex, and province topologies)
- ECS with schema-driven components
- Inventory management (pickup, use, drop)
- Equipment and gear system (wield, wear, inventory slots)
- Combat and damage system
- Job system (job board, query, mutation, AI assignment, events, dependency chains)
- Economic engine (stockpile management, production recipes, resource reservations)
- Procedural generation hooks (worldgen plugins, map validation, postprocessing)
- Lua, Python, and WASM scripting backends with identical API surfaces

The next genre cycle will determine what comes next.
