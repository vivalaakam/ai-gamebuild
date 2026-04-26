#!/usr/bin/env bash
set -euo pipefail

# Setup script for adding ai-rpg-v3 MCP server to Codex CLI
# Run this in your terminal (outside of Codex sandbox)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DB_PATH="$HOME/Library/Application Support/com.vivalaakam.ai-rpg-v3/projects.sqlite"
PROJECT_ID="f573674f-2403-4132-8264-0a7d4ec0a4bd"
MCP_BIN="$PROJECT_ROOT/src-tauri/target/release/mcp"

echo "Adding ai-rpg-v3 MCP server to Codex CLI..."

codex mcp add ai-rpg-v3 -- "$MCP_BIN" \
    --db-path "$DB_PATH" \
    --project-id "$PROJECT_ID"

echo "MCP server 'ai-rpg-v3' added successfully!"
echo ""
echo "Available tools:"
echo "  - get_project    : Get project info"
echo "  - list_scripts   : List all scripts"
echo "  - get_script     : Get a script by id/name"
echo "  - update_script  : Update a script"
echo "  - list_structs   : List all structs"
echo "  - get_struct     : Get a struct by id/name"
echo "  - update_struct  : Update a struct"
echo "  - build_code     : Validate project code"
