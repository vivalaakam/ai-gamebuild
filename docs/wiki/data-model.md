# Data Model

**Summary**: Project state includes scripts, structs, input actions, runtime state, tilesets, and world data.

**Sources**: `src-tauri/src/model.rs`, `src-tauri/src/runtime.rs`.

**Last updated**: 2026-04-26

---

## Project
- `Project` owns scripts, structs, input actions, runtime state, tileset, and world. (source: src-tauri/src/model.rs)
- Demo projects seed scripts, structs, input actions, and world state. (source: src-tauri/src/model.rs)

## World
- `World` contains a tilemap, entities, next ID counter, and camera. (source: src-tauri/src/model.rs)
- `Entity` includes transform, render, flags, optional script binding, and arbitrary state. (source: src-tauri/src/model.rs)

## Tiles
- Tiles are stored in a `Tilemap` with width/height and a flat tile list. (source: src-tauri/src/model.rs)
- `Tileset` metadata describes texture and columns. (source: src-tauri/src/model.rs)

## Input actions
- Input actions are stored with `id`, `label`, and `key_code`. (source: src-tauri/src/model.rs)

## Runtime state
- `runtime_state` stores active entity and player positions in a JSON map. (source: src-tauri/src/model.rs)
- World sync logic aligns runtime state with entity transforms. (source: src-tauri/src/model.rs)

## Related pages
- [[storage]]
- [[runtime-loop]]
