# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PrivateNovelWriter — a private, local-first novel writing assistant with two interfaces:

1. **CLI** — for AI agents to issue commands (JSON output, scriptable)
2. **Windows Desktop App** — Tauri + Svelte, Markdown-based editing

**Tech Stack:** Rust (shared core), Tauri v2 (desktop shell), Svelte 5 (frontend), Markdown (editing format)

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Cargo Workspace                    │
│  ┌───────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │  core/    │  │  cli/    │  │  desktop/        │  │
│  │  (lib)    │◄─┤  (bin)   │  │  (Tauri + Svelte)│  │
│  │           │  │          │  │  ├─ src-tauri/   │  │
│  │           │◄─┤          │  │  │  (Rust backend)│  │
│  │           │  │          │  │  └─ src/         │  │
│  │           │  │          │  │     (Svelte UI)   │  │
│  └───────────┘  └──────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### core/ (Rust library)

Shared logic, no binary. Crate name: `pnw-core`.

- **document/** — Document model, scene/chapter tree, Markdown parsing
- **project/** — Project lifecycle (create, open, save), file layout
- **storage/** — Local file I/O, backup, optional git integration
- **export/** — Export pipeline (EPUB, PDF, HTML, plain text)
- **ai/** — AI agent integration interfaces (tool definitions, command dispatch)
- **config/** — User config, project settings

Design principles:
- All data structures implement `Serialize` + `Deserialize` (serde)
- CLI output types implement a `JsonOutput` trait for `--json` flag
- AI interfaces are trait-based, not coupled to any specific provider

### cli/ (Rust binary)

CLI for AI agent consumption and direct use. Crate name: `pnw`.

Uses `clap` v4 for argument parsing.

```bash
pnw project new <name>          # Create a new novel project
pnw project open <path>         # Open existing project
pnw chapter add <title>         # Add chapter
pnw scene write <id> <text>     # Write content to a scene
pnw scene list                  # List all scenes
pnw export <format> [output]    # Export project
pnw status --json               # Full project status as JSON
pnw ai tool-defs                # Emit tool definitions for AI agent
```

Design rules:
- Every command supports `--json` for machine-parseable output
- Errors use structured error codes (not just exit code 1)
- Idempotent where possible — AI agents may retry on failure
- Use `clap`'s `value_parser` and `subcommand` for type safety

### desktop/ (Tauri v2 + Svelte 5)

Desktop application. Frontend in Svelte 5, backend in Rust using Tauri.

**Frontend (Svelte 5):**
- **Markdown editor** — `codemirror` + markdown plugin, or a minimal custom editor
- **Project tree** — chapter/scene navigator sidebar
- **Preview pane** — rendered Markdown preview (split view or toggle)
- **Status bar** — word count, project stats
- **Settings view** — preferences, export options

**Tauri backend:**
- Thin layer — delegates to `core/` for all business logic
- Exposes Tauri commands that mirror CLI operations
- File dialogs, window management, system tray

## CLI Design for AI Agents

The CLI is a first-class interface for LLM agents. Key contract:

```json
// pnw status --json
{
  "status": "ok",
  "data": {
    "project": {
      "name": "My Novel",
      "word_count": 45000,
      "chapters": [
        { "id": "ch-1", "title": "序幕", "scene_count": 3 }
      ]
    }
  }
}
```

- All JSON output follows a uniform envelope: `{ "status": "ok"|"error", "data": ..., "error": { "code": "...", "message": "..." } }` (when status is "error")
- Custom `pnw ai tool-defs` emits an [anthropic tool definition](https://docs.anthropic.com/en/docs/tool-use) JSON array, so AI agents can discover the available commands dynamically
- Commands that write content accept text via stdin or `--text` flag (AI agents often pipe content)

## Build & Dev Commands

```bash
# Build all workspace crates
cargo build

# Run CLI
cargo run -p pnw -- <command>

# Run desktop app (Tauri dev mode)
cd desktop
cargo tauri dev

# Run tests for all crates
cargo test

# Run tests for specific crate
cargo test -p pnw-core

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt --all
```

## Project Conventions

- Workspace root `Cargo.toml` with members: `core`, `cli`, `desktop/src-tauri`
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `thiserror` for error types, `serde` for serialization
- UI state management in Svelte: use Svelte 5 runes (`$state`, `$derived`, `$effect`)
- IPC between Svelte UI and Tauri backend: use `@tauri-apps/api` `invoke` with type-safe wrappers
- Markdown parsing: use `pulldown-cmark`
- All text content defaults to UTF-8
