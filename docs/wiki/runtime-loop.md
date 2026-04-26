# Runtime Loop

**Summary**: The runtime processes input, dispatches events, runs scripts, and builds a frame each tick.

**Sources**: `src-tauri/src/runtime.rs`, `src-tauri/src/input.rs`, `src-tauri/src/events.rs`, `src-tauri/src/renderer.rs`.

**Last updated**: 2026-04-26

---

## Lifecycle
- `Runtime::open` loads or seeds a project, normalizes demo scripts, and fires `project_load` and `init` events. (source: src-tauri/src/runtime.rs)
- `Runtime::frame` updates input, emits `input`, `update`, and `render` events, then builds a `FrameView`. (source: src-tauri/src/runtime.rs, src-tauri/src/renderer.rs)

## Input processing
- `InputState` maps key codes to actions and produces pressed/released events. (source: src-tauri/src/input.rs)
- Input events are emitted into the runtime event queue. (source: src-tauri/src/runtime.rs)

## Event dispatch
- `EventDispatcher` binds event names to script IDs and manages a queue. (source: src-tauri/src/events.rs)
- Runtime drains the queue with a guard limit to prevent runaway loops. (source: src-tauri/src/runtime.rs)

## Frame rendering
- `build_frame` computes visible tiles/entities and highlights the active entity. (source: src-tauri/src/renderer.rs)
- Logs and draw commands are included in the `FrameView` response. (source: src-tauri/src/renderer.rs)

## Related pages
- [[scripting-api]]
- [[data-model]]
