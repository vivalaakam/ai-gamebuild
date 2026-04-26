# MCP and JSON-RPC

**Summary**: The backend exposes MCP-compatible tools via a JSON-RPC server and a standalone MCP binary.

**Sources**: `src-tauri/src/lib.rs`, `src-tauri/src/jsonrpc.rs`, `src-tauri/src/mcp.rs`.

**Last updated**: 2026-04-26

---

## JSON-RPC server
- Tauri spawns an Axum server on `0.0.0.0:3001/jsonrpc` and shares the runtime state with the GUI. (source: src-tauri/src/lib.rs, src-tauri/src/jsonrpc.rs)
- The JSON-RPC handler exposes MCP-style `tools/list` and `tools/call` plus direct tool methods. (source: src-tauri/src/jsonrpc.rs)

## MCP tool surface
- Tools include `get_project`, `list_scripts`, `get_script`, `update_script`, `list_structs`, `get_struct`, `update_struct`, `build_code`. (source: src-tauri/src/mcp.rs, src-tauri/src/jsonrpc.rs)
- `build_code` validates concatenated structs + scripts via the Rhai engine. (source: src-tauri/src/mcp.rs, src-tauri/src/jsonrpc.rs)

## Standalone MCP server
- CLI `mcp` subcommand starts a server that reads/writes project snapshots from SQLite. (source: src-tauri/src/lib.rs, src-tauri/src/mcp.rs)

## Related pages
- [[scripting-api]]
- [[storage]]
