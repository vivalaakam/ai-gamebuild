# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## MCP Server

The project includes an MCP (Model Context Protocol) server that allows AI coding agents to edit scripts and structs directly.

### Available MCP Tools

- `get_project` — Get current project info (scripts, structs, input actions)
- `list_scripts` — List all scripts with full source
- `get_script` — Get a single script by `id` or `name`
- `update_script` — Update a script source by `id`
- `list_structs` — List all structs with full source
- `get_struct` — Get a single struct by `id` or `name`
- `update_struct` — Update a struct source by `id`
- `build_code` — Validate and build the entire project code

### Setup for Codex CLI

Run in your terminal (outside Codex sandbox):

```bash
./scripts/setup-codex-mcp.sh
```

Or manually:

```bash
codex mcp add ai-rpg-v3 -- \
  "$(pwd)/src-tauri/target/release/mcp" \
  --db-path "$HOME/Library/Application Support/com.vivalaakam.ai-rpg-v3/projects.sqlite" \
  --project-id "f573674f-2403-4132-8264-0a7d4ec0a4bd"
```

Verify with `codex mcp list`.

### Run via Tauri CLI Plugin

```bash
cd src-tauri
cargo run -- mcp \
  --db-path "$HOME/Library/Application Support/com.vivalaakam.ai-rpg-v3/projects.sqlite" \
  --project-id "YOUR-PROJECT-ID"
```

### Standalone MCP Binary

```bash
cd src-tauri
cargo build --bin mcp --release

./target/release/mcp \
  --db-path "$HOME/Library/Application Support/com.vivalaakam.ai-rpg-v3/projects.sqlite" \
  --project-id "YOUR-PROJECT-ID"
```
