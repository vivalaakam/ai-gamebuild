# Storage

**Summary**: SQLite persistence stores project snapshots and normalized tables for scripts, structs, and world data.

**Sources**: `src-tauri/src/storage.rs`.

**Last updated**: 2026-04-26

---

## Schema
- Tables include `projects`, `scripts`, `structs`, `tilesets`, `tilemaps`, `entities`, `input_actions`, `runtime_state`. (source: src-tauri/src/storage.rs)
- `projects.snapshot` stores the full serialized project blob. (source: src-tauri/src/storage.rs)

## Loading
- `load_or_seed` returns the latest project or seeds a demo project. (source: src-tauri/src/storage.rs)
- `apply_db_overrides` replaces parts of the snapshot with table data when present. (source: src-tauri/src/storage.rs)

## Saving
- `save_snapshot` writes the project blob and upserts normalized tables. (source: src-tauri/src/storage.rs)

## Related pages
- [[data-model]]
- [[runtime-loop]]
