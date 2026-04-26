# context-mode — MANDATORY routing rules

You have context-mode MCP tools available. These rules are NOT optional — they protect your context window from flooding. A single unrouted command can dump 56 KB into context and waste the entire session. Codex CLI does NOT have hooks, so these instructions are your ONLY enforcement mechanism. Follow them strictly.

## Think in Code — MANDATORY

When you need to analyze, count, filter, compare, search, parse, transform, or process data: **write code** that does the work via `ctx_execute(language, code)` and `console.log()` only the answer. Do NOT read raw data into context to process mentally. Your role is to PROGRAM the analysis, not to COMPUTE it. Write robust, pure JavaScript — no npm dependencies, only Node.js built-ins (`fs`, `path`, `child_process`). Always use `try/catch`, handle `null`/`undefined`, and ensure compatibility with both Node.js and Bun. One script replaces ten tool calls and saves 100x context.

## BLOCKED commands — do NOT use these

### curl / wget — FORBIDDEN
Do NOT use `curl` or `wget` in any shell command. They dump raw HTTP responses directly into your context window.
Instead use:
- `ctx_fetch_and_index(url, source)` to fetch and index web pages
- `ctx_execute(language: "javascript", code: "const r = await fetch(...)")` to run HTTP calls in sandbox

### Inline HTTP — FORBIDDEN
Do NOT run inline HTTP calls via `node -e "fetch(..."`, `python -c "requests.get(..."`, or similar patterns. They bypass the sandbox and flood context.
Instead use:
- `ctx_execute(language, code)` to run HTTP calls in sandbox — only stdout enters context

### Direct web fetching — FORBIDDEN
Do NOT use any direct URL fetching tool. Raw HTML can exceed 100 KB.
Instead use:
- `ctx_fetch_and_index(url, source)` then `ctx_search(queries)` to query the indexed content

## REDIRECTED tools — use sandbox equivalents

### Shell (>20 lines output)
Shell is ONLY for: `git`, `mkdir`, `rm`, `mv`, `cd`, `ls`, `npm install`, `pip install`, and other short-output commands.
For everything else, use:
- `ctx_batch_execute(commands, queries)` — run multiple commands + search in ONE call
- `ctx_execute(language: "shell", code: "...")` — run in sandbox, only stdout enters context

### File reading (for analysis)
If you are reading a file to **edit** it → reading is correct (edit needs content in context).
If you are reading to **analyze, explore, or summarize** → use `ctx_execute_file(path, language, code)` instead. Only your printed summary enters context. The raw file stays in the sandbox.

### grep / search (large results)
Search results can flood context. Use `ctx_execute(language: "shell", code: "grep ...")` to run searches in sandbox. Only your printed summary enters context.

## Tool selection hierarchy

1. **GATHER**: `ctx_batch_execute(commands, queries)` — Primary tool. Runs all commands, auto-indexes output, returns search results. ONE call replaces 30+ individual calls. Each command: `{label: "descriptive header", command: "..."}`. Label becomes FTS5 chunk title — descriptive labels improve search.
2. **FOLLOW-UP**: `ctx_search(queries: ["q1", "q2", ...])` — Query indexed content. Pass ALL questions as array in ONE call.
3. **PROCESSING**: `ctx_execute(language, code)` | `ctx_execute_file(path, language, code)` — Sandbox execution. Only stdout enters context.
4. **WEB**: `ctx_fetch_and_index(url, source)` then `ctx_search(queries)` — Fetch, chunk, index, query. Raw HTML never enters context.
5. **INDEX**: `ctx_index(content, source)` — Store content in FTS5 knowledge base for later search.

## Output constraints

- Keep responses under 500 words.
- Write artifacts (code, configs, PRDs) to FILES — never return them as inline text. Return only: file path + 1-line description.
- When indexing content, use descriptive source labels so others can `search(source: "label")` later.

## ctx commands

| Command | Action |
|---------|--------|
| `ctx stats` | Call the `stats` MCP tool and display the full output verbatim |
| `ctx doctor` | Call the `doctor` MCP tool, run the returned shell command, display as checklist |
| `ctx upgrade` | Call the `upgrade` MCP tool, run the returned shell command, display as checklist |
| `ctx purge` | Call the `purge` MCP tool with confirm: true. Warns before wiping the knowledge base. |

After /clear or /compact: knowledge base and session stats are preserved. Use `ctx purge` if you want to start fresh.

## Windows notes

**PowerShell cmdlets in shell scripts** — The sandbox executes scripts via bash. PowerShell
cmdlets (`Format-List`, `Format-Table`, `Get-Culture`, etc.) do not exist in bash and will fail
with `command not found`. Wrap them with `pwsh -NoProfile -Command "..."` instead.

**Relative paths** — The sandbox CWD is a temp directory, not your project root. Always convert
any user-supplied path to an absolute path before passing it to `rg`, `grep`, or `find`.
Ask the user to confirm the absolute path if it is not already known.

**Windows drive letter paths** — The sandbox runs Git Bash / MSYS2, not WSL. Drive letters must
use the MSYS2 convention, NOT the WSL convention:
`X:\path` → `/x/path` (lowercase letter, no `/mnt/` prefix).
Never emit `/mnt/<letter>/` prefixes regardless of which drive the project is on.

**Always quote paths** — If a path contains spaces, bash splits it into separate arguments.
Always wrap every path in double quotes: `rg "symbol" "$REPO_ROOT/some dir/Source"`.
This applies to all tools: `rg`, `grep`, `find`, `ls`, etc.

OpenViking Execution Protocol

## 0. Purpose

This agent operates with OpenViking as its **primary memory and context system**.

The agent MUST:
- Use OpenViking for all long-term memory and knowledge
- Retrieve context before acting
- Persist all meaningful results after acting

> If information is not stored in OpenViking, it is considered non-existent.

---

## 1. System Model

OpenViking is a **filesystem-based context database** that unifies:
- memory
- resources
- skills

All data is organized via `viking://` URIs in a hierarchical structure :contentReference[oaicite:1]{index=1}

### Canonical structure:


viking://
├── resources/ # code, docs, external data
├── agent/ # memory, experience, reasoning
├── user/ # preferences, constraints
├── skills/ # reusable logic patterns


---

## 2. Core Principle

The agent is NOT stateless.

The agent is a **stateful system backed by OpenViking**.

Every task must follow:


retrieve → analyze → execute → persist


---

## 3. Retrieval Rules (MANDATORY)

Before performing any action, the agent MUST retrieve context.

### 3.1 Determine scope

| Task type | Path |
|----------|------|
| Code-related | viking://resources/code/ |
| Documentation | viking://resources/docs/ |
| Previous work | viking://agent/memory/ |
| User context | viking://user/ |

---

### 3.2 Retrieval operations

Use filesystem-style navigation:


ls(viking://...)
find(viking://..., query)
read(viking://...)
overview(viking://...)


---

### 3.3 Tiered loading strategy

OpenViking supports L0/L1/L2 context levels :contentReference[oaicite:2]{index=2}

The agent MUST:

- Start with **L0 (abstract)**
- Expand to **L1 (structure)**
- Load **L2 (details)** only if required

---

## 4. Execution Rules

After retrieval, the agent:

1. Builds context
2. Identifies dependencies
3. Executes task

The agent MUST:
- avoid assumptions
- rely on retrieved context
- trace decisions to sources

---

## 5. Persistence Rules (CRITICAL)

After every non-trivial task, the agent MUST persist results.

### 5.1 What must be stored

- solutions
- reasoning outcomes
- architectural decisions
- bug fixes
- code changes
- discovered relationships

---

### 5.2 Storage format


write(viking://agent/memory/<topic>/<id>.md)

content:

context: task description
reasoning: decision process
result: final outcome
references:
viking://resources/...

---

### 5.3 Update vs Create

- If similar entry exists → UPDATE
- If new knowledge → CREATE

Never duplicate memory.

---

## 6. Code-Aware Behavior

When working with code:

### BEFORE:
- locate file in `viking://resources/code/`
- analyze dependencies
- check previous changes

### AFTER:
Persist change log:


viking://agent/memory/code_changes/<file>.md


Include:
- what changed
- why
- impact

---

## 7. Knowledge Linking

All stored knowledge MUST be connected.

Example:


references:

viking://resources/code/auth/service.ts
viking://agent/memory/bugfix/auth-timeout

OpenViking relies on **hierarchical + relational context**, not flat storage :contentReference[oaicite:3]{index=3}

---

## 8. Anti-Patterns (FORBIDDEN)

The agent MUST NOT:

- skip retrieval
- skip persistence
- duplicate knowledge
- rely only on prompt context
- ignore OpenViking

Violating these rules breaks system consistency.

---

## 9. Execution Loop (STRICT)

Every task MUST follow:

Parse task
Retrieve context (OpenViking)
Analyze
Execute
Persist result (OpenViking)
Link knowledge

---

## 10. Memory Evolution

OpenViking supports persistent and evolving memory across sessions :contentReference[oaicite:4]{index=4}

The agent MUST:

- improve existing records
- refine previous reasoning
- consolidate duplicate knowledge

---

## 11. Skill Extraction

If a pattern repeats:


save → viking://skills/<skill>.md


Skills must be:
- reusable
- minimal
- well-scoped

---

## 12. Quality Constraints

Each stored entry must be:

- concise
- structured
- linked
- reusable
- context-rich

---

## 13. Final Rule

The agent does not "remember".

OpenViking remembers.

The agent **maintains and evolves that memory system**.
---

## MCP Server — ai-rpg-v3

This project exposes an MCP (Model Context Protocol) server for AI coding agents to edit scripts and structs directly.

### Available Tools

| Tool | Description |
|------|-------------|
| `get_project` | Get current project info (scripts, structs, input actions) |
| `list_scripts` | List all scripts with full source |
| `get_script` | Get a single script by `id` or `name` |
| `update_script` | Update a script source by `id` |
| `list_structs` | List all structs with full source |
| `get_struct` | Get a single struct by `id` or `name` |
| `update_struct` | Update a struct source by `id` |
| `build_code` | Validate and build the entire project code |

---

## LLM Wiki System (docs/)

This repo uses the LLM Wiki pattern to maintain living documentation.

### Folder structure

```
docs/raw/   -- source documents (immutable, never edit)
docs/wiki/  -- markdown wiki pages maintained by the agent
docs/wiki/index.md -- table of contents
docs/wiki/log.md   -- append-only operations log
```

### Page format (required)

Every page in `docs/wiki/` must follow:

```markdown
# Page Title

**Summary**: One to two sentences describing this page.

**Sources**: List of raw/code files this page draws from.

**Last updated**: YYYY-MM-DD

---

Main content, use headings and short paragraphs.

## Related pages
- [[related-page]]
```

### Ingest workflow

When the user adds a new source to `docs/raw/` and asks to ingest it:

1. Read the full source file.
2. Discuss key takeaways with the user before writing anything.
3. Create a summary page in `docs/wiki/` named after the source.
4. Create/update related concept pages with `[[wiki-links]]`.
5. Update `docs/wiki/index.md` with new pages and one-line descriptions.
6. Append an entry to `docs/wiki/log.md` with date + what changed.

### Query workflow

1. Read `docs/wiki/index.md` first to find relevant pages.
2. Read relevant pages and synthesize an answer with citations.
3. If the answer is valuable, offer to save it as a new wiki page.

### Lint workflow

When asked to lint/audit the wiki:

- Check for contradictions between pages.
- Find orphan pages (no inbound links).
- Identify concepts mentioned without their own page.
- Flag claims that may be outdated.
- Ensure pages follow the page format above.

### Rules

- Never modify anything in `docs/raw/`.
- Always update `docs/wiki/index.md` and `docs/wiki/log.md` after changes.
- Keep page names lowercase with hyphens (e.g. `runtime-loop.md`).
