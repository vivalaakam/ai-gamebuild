# Architecture

**Summary**: The system is a Tauri UI front-end backed by a Rust runtime, scripting engine, and SQLite storage.

**Sources**: `src/App.tsx`, `src-tauri/src/lib.rs`, `src-tauri/src/runtime.rs`, `src-tauri/src/scripting.rs`, `src-tauri/src/storage.rs`.

**Last updated**: 2026-04-26

---

## Components
- **Frontend UI**: React app with Monaco editor, file tree, runtime preview, and console. (source: src/App.tsx)
- **Runtime**: Owns project state, input state, event dispatcher, and per-frame execution. (source: src-tauri/src/runtime.rs)
- **Scripting**: Rhai engine with registered functions for tiles, entities, events, and state. (source: src-tauri/src/scripting.rs)
- **Renderer**: Builds frame views from world state and draw commands. (source: src-tauri/src/renderer.rs)
- **Storage**: SQLite persistence of projects, scripts, structs, entities, tilemaps, and runtime state. (source: src-tauri/src/storage.rs)

## Data flow (per frame)
1. Frontend sends `pressedKeys` and `delta` to `run_frame`. (source: src/App.tsx, src-tauri/src/lib.rs)
2. Runtime updates input state and emits `input`, `update`, `render` events. (source: src-tauri/src/runtime.rs)
3. Script runtime executes bound scripts and collects events/logs/draw commands. (source: src-tauri/src/scripting.rs)
4. Renderer computes visible tiles/entities and returns a `FrameView`. (source: src-tauri/src/renderer.rs)
5. Frontend draws frame to the canvas. (source: src/App.tsx)

## Integration surfaces
- Tauri commands for UI interactions. (source: src-tauri/src/lib.rs)
- JSON-RPC server on `:3001` exposing MCP-style tools. (source: src-tauri/src/lib.rs, src-tauri/src/jsonrpc.rs)
- Standalone MCP server binary for CLI tools. (source: src-tauri/src/lib.rs, src-tauri/src/mcp.rs)

## Related pages
- [[frontend-app]]
- [[runtime-loop]]
- [[scripting-api]]
- [[storage]]
