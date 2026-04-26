# Scripting API

**Summary**: Rhai scripts run per event and can mutate state, spawn entities, and emit events.

**Sources**: `src-tauri/src/scripting.rs`, `src-tauri/src/model.rs`.

**Last updated**: 2026-04-26

---

## Script execution
- Scripts are compiled per-dispatch with dependency resolution and shared structs. (source: src-tauri/src/scripting.rs)
- Each event invokes `on_<event>` functions, e.g. `on_input`. (source: src-tauri/src/scripting.rs)

## Built-in functions (selected)
- **State**: `state_get`, `state_set`, `state_remove`. (source: src-tauri/src/scripting.rs)
- **Tiles**: `get_tile`, `set_tile`, `clear`, `draw_tile`. (source: src-tauri/src/scripting.rs)
- **Entities**: `entity_spawn_raw`, `entity_set_pos_raw`, `entity_next_id`, `get_entity`, `entity_ids`, `remove_entity`. (source: src-tauri/src/scripting.rs)
- **Input**: `is_pressed`, `is_just_pressed`. (source: src-tauri/src/scripting.rs)
- **Events/logs**: `emit`, `log`, `draw_entity`. (source: src-tauri/src/scripting.rs)

## Script sources
- Structs are prepended to every script to form shared definitions. (source: src-tauri/src/scripting.rs, src-tauri/src/model.rs)
- Built-in libraries are stored as scripts with empty bindings so they load for all scripts. (source: src-tauri/src/model.rs, src-tauri/src/scripting.rs)

## Related pages
- [[data-model]]
- [[runtime-loop]]
