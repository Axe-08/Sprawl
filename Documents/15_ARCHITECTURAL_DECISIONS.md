# Sprawl — Architectural Decisions & Executive Log

> **Maintained by**: Antigravity  
> **Last Updated**: 2026-07-07  
> This document is the authoritative record of every key architectural decision, tradeoff accepted, or design direction confirmed during the Sprawl project. New contributors should read this before touching any crate.

---

## ADR-001 — No C Dependencies at MVP (Mock-Backend Feature Gate Pattern)

**Decision**: Avoid native C-library dependencies (lancedb, candle GGUF loader) at MVP. Use `#[cfg(any(test, feature = "mock-backend"))]` gates to compile mock implementations in test/dev builds while keeping production paths clean.

**Rationale**: C dependencies require system headers, pkg-config, and cross-compilation toolchains. For a cross-platform (Linux/macOS/Windows) project targeting contributors on any OS, requiring libclang or libtorch at build time creates unnecessary friction.

**Tradeoff accepted**: `sprawl-archivist` and `sprawl-inference` contain scaffold stubs (zero-vector embeddings, mock GGUF download). These are explicitly documented as Phase 2 replacements, not production code.

**Rule**: Any new mock must live in `sprawl-dev` or behind a `cfg` gate. Never in an unconditional production path.

---

## ADR-002 — `sprawl-dev` Crate for All Development Fixtures

**Decision**: All mock backends, demo data, and development-only fixture helpers live exclusively in the `crates/sprawl-dev/` crate. This crate is **never** a dependency of production crates.

**Rationale**: Early development had mock structs (`MockKeyringStore`, `MockDatabase`, `MockLedger`) scattered inline within `sprawl-sentinel`, `sprawl-archivist`, etc. This caused confusion about what was real vs. fake and polluted the production module tree visible to contributors.

**Rule for contributors**: If you're writing a struct or function that only exists to support testing or demonstration, it goes in `sprawl-dev`, not in the feature crate itself.

**What's in `sprawl-dev`**:
- `MockDatabase` (implements `VectorDatabase`)
- `MockKeyringStore` (implements `KeyringBackend`)
- `MockLedger` (implements `LedgerBackend`)
- `HighRamMock` (implements `SysInfo`)
- `data.rs` — demo triage items and dashboard strings

---

## ADR-003 — Resurrect Has No Execution Surface

**Decision**: The `resurrect` command (`sprawl resurrect --export-agent`) produces a `resurrection-kit.md` file and copies it to clipboard. It never executes Docker, shell scripts, or any external process.

**Rationale**: Executing arbitrary code resurrected from a dead project is a significant security surface. If Docker is unavailable, output the kit as text and hard-exit rather than silently falling back to a shell-exec path.

**Rule**: Any PR that adds `std::process::Command` inside `resurrect`'s code path is rejected.

---

## ADR-004 — 0% CPU Idle (No Poll Loop in TUI or Daemon)

**Decision**: Both the TUI and the daemon must not spin-poll. The TUI uses `crossterm::event::poll` with an indefinite timeout. The daemon uses `notify`'s blocking channel.

**Rationale**: NFR-1 requires "~0% CPU while idle." A 100ms poll loop consumes ~1-3% CPU constantly. Developer machines get hot, batteries drain.

**Implementation**: `event::poll(Duration::from_secs(60))` — blocks until a key/mouse event arrives or a 60-second keepalive timeout. No `sleep` loops anywhere in the hot path.

---

## ADR-005 — Safety Gate Is Conservative (Core Always Wins)

**Decision**: `SafetyGate.nuke_eligible()` uses a conservative merge — if either core check or plugin check indicates not reproducible, the result is `Locked`. A plugin can never override a core veto.

**Rationale**: Safety gate failures (Rows 2, 3, 4, 6) are release-blocking security defects. A plugin that hallucinates "safe" on a project with local patches must not unlock a Nuke. Core veto is absolute.

**Rule**: The 7 safety gate matrix rows must always pass. Any regression on rows 2/3/4/6 blocks the release.

---

## ADR-006 — Config System Is 4-Layer, File-Based at Merge Time

**Decision**: Configuration is merged in four layers (Global defaults → Team → Project local → CLI flags) at load time. The `source` field on every `SweepRule` tracks which layer added/overrode it.

**Status at MVP**: Layers 1 (global defaults) and 2 (persona preset) are wired. Layers 3 (disk-based team `.sprawl.toml`) and 4 (project `.sprawl.toml`) are architecturally defined but file-based auto-discovery is not yet complete.

---

## ADR-007 — WASM Plugins Are Sandboxed, Fuel-Limited, Crash-Safe

**Decision**: All third-party plugins run inside a `wasmtime` sandbox with:
- Read-only preopened dir at `/project` only
- Fuel limit: 1,000,000,000 instructions
- Plugin crash → logged as `warn!`, not a panic

**Rationale**: Community plugins from untrusted sources must not be able to read secrets, write files, or hang the daemon forever.

---

## ADR-008 — No Auto-LLM on Passive Scan

**Decision**: The daemon's passive file watch and Sentinel L1 scan never cold-boot the LLM automatically. Any LLM invocation requires:
1. Explicit user consent (`[W]` batch classify in TUI, or `analyze --deep`)
2. A RAM pre-flight check (`INSUFFICIENT HEADROOM: XMB required, YMB available`)

**Rationale**: Cold-booting Phi-3 takes 3-8 seconds and requires 3GB+ of RAM. Doing this silently in the background would be hostile UX and violates NFR-5.

---

## ADR-009 — Mock-Backend Feature Gate Pattern

**Decision**: Any crate that needs a mock implementation for testing defines:
```toml
[features]
mock-backend = []
```
And gates mocks like:
```rust
#[cfg(any(test, feature = "mock-backend"))]
pub struct MockFoo;
```

The `sprawl-dev` crate activates `mock-backend` on all relevant crates as its dependencies. Production binaries never link with `mock-backend` unless explicitly requested.

**Rule**: `sprawl-cli` has both `mock-backend` and `debug` features, where `debug = ["mock-backend"]`. The `[dev] mock_data = true` flag in `.sprawl.toml` is parsed by `sprawl-cli/src/config.rs`.

---

## ADR-010 — TUI Minimum Size Is 80×24, Degradation Order

**Decision**: Below 80×24, the TUI shows a warning banner and blocks rendering. At that boundary, degradation priority is:
1. Truncate long paths with middle ellipsis first
2. Drop secondary metadata columns
3. Keep the hotkey footer until last

**Rationale**: The hotkey footer is the user's only way to navigate — it must be the last thing dropped.

---

## ADR-011 — Mouse: Scroll Only, No Click Actions

**Decision**: `EnableMouseCapture` is set but only `MouseEventKind::ScrollUp` and `ScrollDown` are handled. Click events are explicitly ignored.

**Rationale**: Click action handling requires precise hit-testing of rendered widget areas. This creates a testing surface that cannot be easily unit-tested and introduces fragility at different terminal sizes. Keyboard bindings are the primary interaction model.

---

## Chronological Decision Timeline

| Date | Decision | Trigger |
|------|----------|---------|
| Session 1-3 | Scaffolded full workspace (M1-M10) | Initial implementation |
| Session 4 | Extracted `VectorDatabase`, `KeyringBackend`, `LedgerBackend` traits | Mock data cleanup request |
| Session 4 | Created `sprawl-dev` crate | User request: "clean codebase for contributors" |
| Session 4 | Added `mock-backend` feature gate pattern | ADR-001 / ADR-009 |
| Session 5 | Implemented TUI (M11) with `ratatui` + `crossterm` | M11 implementation |
| Session 5 | Gated TUI demo items behind `demo-data` feature flag | Mock isolation completion |
| Session 5 | Added `sprawl ui` CLI subcommand | CLI wiring |
| Session 5 | Added `[dev] mock_data` TOML config parse | User direction on debug toggle |

---

## What Is Still a Known Stub (Not a Bug, a Decision)

| Crate | Stub | Phase |
|-------|------|-------|
| `sprawl-inference` | `run_prompt` / `load_model` — no actual Candle GGUF loading | Phase 2 |
| `sprawl-archivist` | `search` uses zero-vector embedding, no real lancedb | Phase 2 |
| `sprawl-sentinel/verify.rs` | MCP-routed verification is a simulated response | Phase 2 |
| `sprawl-daemon/lib.rs` | Event dispatch to Archaeologist/Sweeper/Sentinel not wired | Phase 2 |
| `sprawl-sweeper/engine.rs` | `restore` reads from manifest but manifest persistence is partial | M12 |

---

## ADR-012 — Daemon Start is Foreground Blocking (MVP)

**Decision**: `sprawl daemon start` runs synchronously in the foreground, blocking the terminal. It does not `fork()` into the background.

**Rationale**: Implementing a true daemonizing `fork()` that works consistently across Windows and Unix is complex and unnecessary for MVP. We rely on the user or an OS-level service manager (`systemd`, `launchd`) to background the process.

**Future Work**: Post-MVP, a true cross-platform backgrounding mechanism will be added so users can spawn it and get their shell back immediately.

---

## ADR-013 — Plugin Installation is Local-Only (MVP)

**Decision**: `sprawl plugin install <path>` only accepts local file paths to `.wasm` files. URLs are explicitly rejected.

**Rationale**: Supporting URL downloads requires shipping HTTP clients, dealing with TLS, network errors, and crucially, enforcing cryptographic signature checks to prevent supply-chain attacks. Local-only ensures the user is deliberately installing a binary they fetched themselves.

**Future Work**: Post-MVP, we will support URLs and potentially build a Sprawl Plugin Marketplace with integrated signing/verification.

---

## ADR-014 — `--json` Scope is Limited to Status/Info Commands

**Decision**: The `--json` CLI flag is only respected by commands that output structured operational data (`daemon status`, `plugin list`, `verify`, `analyze`). 

**Rationale**: Forcing `--json` on commands like `bundle` (which outputs a 10,000-line markdown file) creates bloated, hard-to-parse responses wrapped in JSON envelopes. 

**Future Work**: Post-MVP, this may be expanded to provide custom metadata structures (e.g., `bundle --json` returning the local path to the generated markdown file rather than the contents).

---

## ADR-015 — Candle Inference Feature Gate (Compile Times)

**Decision**: The `candle` ecosystem (core, nn, transformers) and `tokenizers` dependencies are gated behind an optional `inference` feature flag in `sprawl-inference`.

**Rationale**: `candle` is a massive dependency tree that significantly impacts compile times. For routine development of the TUI, sweeping logic, or plugins, forcing developers to compile the inference engine creates friction.

**Tradeoff accepted**: The default `cargo build` produces a binary that cannot run actual LLM inference (falling back to mock responses or failing gracefully). True inference requires `cargo build --features inference`.

---

## ADR-016 — Windows MCP Verify Fallback (Unknown)

**Decision**: The `sprawl verify` command uses `std::os::unix::net::UnixStream` on Unix-like systems to communicate with the MCP server. On Windows, it explicitly stubs out to `VerificationStatus::Unknown`.

**Rationale**: Windows does not support `UnixStream`. While named pipes (`\\.\pipe\...`) could be used, there is currently no established standard for MCP servers on Windows that we can reliably delegate to for MVP.

**Future Work**: Post-MVP, implement named pipe IPC for Windows once the MCP ecosystem matures on that platform.
