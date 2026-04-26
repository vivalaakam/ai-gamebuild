# Frontend App

**Summary**: The frontend is a React + Monaco editor shell that edits Rhai files and renders a live preview.

**Sources**: `src/App.tsx`, `src/main.tsx`.

**Last updated**: 2026-04-26

---

## UI layout
- Left rail lists scripts, structs, and input actions with selection and creation controls. (source: src/App.tsx)
- Main editor uses Monaco with Rhai language mode. (source: src/App.tsx)
- Runtime preview draws a canvas frame from backend `FrameView`. (source: src/App.tsx)
- Console panel shows status and runtime logs. (source: src/App.tsx)

## Runtime interaction
- On boot, the app loads the project via `load_project` and sets initial selection. (source: src/App.tsx)
- Edits are validated via `validate_script` and applied via `update_script` / `update_struct`. (source: src/App.tsx)
- A `requestAnimationFrame` loop calls `run_frame` and paints the canvas. (source: src/App.tsx)
- Input action binding captures keypresses and updates `update_input_action`. (source: src/App.tsx)

## Editor behavior
- The editor auto-selects markers like `name_fn` and `make_name` after creating new units. (source: src/App.tsx)
- Validation status is shown next to the editor title. (source: src/App.tsx)

## Related pages
- [[runtime-loop]]
- [[scripting-api]]
