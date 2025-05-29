# MVP Roguelike Mod for MGE

This mod demonstrates a minimal roguelike game using MGE's ECS, schema-driven components, and Lua scripting.

## Features

- Player, monster, and item entities defined by schemas
- Map loaded from JSON asset
- Turn-based movement and combat
- Simple rendering in terminal
- Camera/viewport follows the player

## How to Run

1. Ensure this mod is in your `mods/` directory.
2. Launch MGE with mod loading enabled.
3. The game will start in the terminal. Use `WASD` to move, `q` to quit.

## File Structure

- `mod.json` - Mod manifest
- `schemas/` - ECS component schemas
- `assets/map1.json` - Map definition
- `systems/main.lua` - Main game logic
