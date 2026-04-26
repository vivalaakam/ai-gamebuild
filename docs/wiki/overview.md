# Overview

**Summary**: ai-rpg-v3 is a Tauri desktop app that edits Rhai scripts and previews a tiny RPG runtime.

**Sources**: `src/App.tsx`, `src-tauri/src/lib.rs`, `src-tauri/src/runtime.rs`.

**Last updated**: 2026-04-26

---

## Product snapshot
The app is a Tauri + React desktop UI that loads a project, edits scripts/structs, and streams a runtime preview to a canvas. (source: src/App.tsx, src-tauri/src/lib.rs)

## Core capabilities
- Script/struct editing with validation before applying changes in memory. (source: src/App.tsx, src-tauri/src/runtime.rs)
- Runtime preview rendered from the backend frame loop. (source: src/App.tsx, src-tauri/src/runtime.rs)
- Input action binding and reset controls surfaced in the UI. (source: src/App.tsx, src-tauri/src/runtime.rs)
- Project persistence via snapshot storage. (source: src/App.tsx, src-tauri/src/runtime.rs)

## Key subsystems
- Frontend editor shell and preview canvas. (source: src/App.tsx)
- Backend runtime that dispatches events and builds frames. (source: src-tauri/src/runtime.rs, src-tauri/src/renderer.rs)
- Rhai scripting host with game-specific APIs. (source: src-tauri/src/scripting.rs)
- SQLite-backed persistence for project data. (source: src-tauri/src/storage.rs)

## Related pages
- [[architecture]]
- [[frontend-app]]
- [[runtime-loop]]
