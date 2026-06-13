# CODEBUDDY.md

This file provides guidance to CodeBuddy Code when working with code in this repository.

## Project Overview

PrivateNovelWriter — a private, local-first novel writing assistant designed for AI-agent collaboration. The system uses a three-layer tool design:

- **Agent A** (user's AI assistant, e.g. Claude) interacts via the **CLI**
- **Agent B** (built-in writing expert AI) handles creative orchestration via LLM calls
- **Data layer** stores everything in **SQLite + plain .txt files**

Currently implements phases P1-P3 (data model, SQLite + .txt storage, CLI command set, Agent B writing expert). Phases P4-P5 (desktop app, bulk/export features) are not yet started.

**Tech Stack:** Rust (workspace), SQLite (rusqlite bundled), .txt plain text storage, LLM integration via OpenAI-compatible API (DeepSeek + LM Studio).

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Cargo Workspace                         │
│  ┌────────────────────┐    ┌────────────────────────┐   │
│  │   kernel/ (lib)     │    │    cli/ (bin)          │   │
│  │   pnw-kernel        │◄───┤    pnw                 │   │
│  │                     │    │                        │   │
│  │  models/            │    │  main.rs (clap CLI)    │   │
│  │  db/ (SQLite)       │    │  - 9 subcommand groups │   │
│  │  storage/ (.txt I/O)│    │  - JSON output support │   │
│  │  command/ (enums)   │    │  - Sync CRUD handlers  │   │
│  │  handler/ (execute) │    │  - Async agent handlers│   │
│  │  agent/ (LLM)       │    │                        │   │
│  └────────────────────┘    └────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### kernel/ (Rust library, crate: `pnw-kernel`)

Shared logic, no binary. Modules:

- **models/** — Domain types: `Novel`, `NovelSetting`, `Character`, `OutlinePhase/Chapter`, `TextPhase/Chapter`, `Plugin`, `DetailSample`. All `Serialize` + `Deserialize` with serde.
- **db/** — SQLite layer. `schema.rs` creates 9 tables. `crud.rs` has full CRUD for all entities.
- **storage/** — .txt file I/O: `read_text`, `write_text`, `delete_file`, `count_chars` (non-whitespace char counting for Chinese text).
- **command/** — Typed command enums. `DataCommand` (all CRUD ops), `AgentCommand` (`WriteChapter`, `ReviseChapter`, `PlanOutline`, `Evaluate`), unified `Command` enum.
- **handler/** — `Handler` struct with `conn: Connection` + `project_path`. `execute(DataCommand) -> Result<Output>` is the single dispatch point handling all data commands.
- **agent/** — LLM integration. `llm.rs`: OpenAI-compatible provider trait + `OpenAiCompatible` implementation (supports DeepSeek API and LM Studio). `write_chapter.rs`: writes novel body with character/sample context. `revise.rs`: revision with feedback. `plan.rs`: outline generation from LLM output (parses `|`-delimited lines). `evaluate.rs`: quality assessment on 4 dimensions.

### cli/ (Rust binary, crate: `pnw`)

Single-file CLI at `main.rs` (~870 lines). Uses `clap` v4 derive.

**9 subcommand groups:**
- `novel` — `new` (creates project dir + `project.db` + `text/`), `open`, `list`, `show`, `config`
- `setting` — `show`, `update`
- `character` — `add`, `list`, `get`, `update`, `delete`
- `plugin` — `show`, `set`, `delete`
- `outline` — `show`, `phase` subcommands, `chapter` subcommands
- `text` — `phase` subcommands, `chapter` subcommands (create links outline chapter, auto-generates file path)
- `sample` — `add`, `list`, `delete`
- `status` — global progress summary with word count, completion percentage
- `chapter` — `write`, `revise` (agent commands)
- `evaluate` — quality evaluation

### Project Layout on Disk

```
my-novel/
├── project.db       # SQLite database
└── text/            # Chapter text files
    ├── phase-name-1/
    │   ├── ch-001.txt
    │   └── ch-002.txt
    └── phase-name-2/
        └── ch-003.txt
```

## Build & Development Commands

```bash
# Build all workspace crates
cargo build

# Build specific crate
cargo build -p pnw
cargo build -p pnw-kernel

# Run the CLI
cargo run -p pnw -- <command>

# Run all tests
cargo test

# Run tests for specific crate
cargo test -p pnw-kernel

# Run a single test function by name
cargo test -p pnw-kernel -- <test_name>

# Lint (matches CI)
cargo clippy --all-targets -- -D warnings

# Format check (matches CI)
cargo fmt --all --check

# Format code
cargo fmt --all
```

### CI Pipeline (`.github/workflows/ci.yml`)

Runs on `windows-latest` with `stable-x86_64-pc-windows-gnu` target triple. Steps: build, test, fmt check, clippy.

## Environment Setup

A `.env` file is required at workspace root. See `.env.example` for template:

```env
LLM_PROVIDER=deepseek      # "deepseek" (api.deepseek.com) or "lmstudio" (localhost:1234/v1)
DEEPSEEK_API_KEY=sk-xxx    # Only needed for deepseek provider
```

## CLI Design for AI Agents

- Every command supports `--json` flag for machine-parseable output
- JSON envelope format: `{ "status": "ok"|"error", "data": ..., "error": { "code": "...", "message": "..." } }`
- Text content can be provided via file arg, `--text` flag, or stdin
- `pnw status --json` provides full project snapshot in one call

## Project Conventions

- Workspace root `Cargo.toml` with `members = ["kernel", "cli"]`, resolver 2
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `thiserror` for error types, `serde` for serialization
- All strings default to UTF-8
- LLM provider abstraction via `LlmProvider` trait in `agent::llm` (supports both streaming and non-streaming)
- `novel new` creates the full project structure (dir + db + subdirs) in one command — no separate `init` step needed
- Outline chapter ↔ text chapter linking: `text chapter create --from-outline <id>` establishes the relationship; text chapter `file_path` auto-generated as `text/{phase_name}/ch-{sort:03}.txt`
- Config for agent operations lives in `.env`, not in the project DB
