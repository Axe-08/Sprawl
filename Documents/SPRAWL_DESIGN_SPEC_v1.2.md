# Sprawl — Master Architectural Specification (v1.2, Final Pre-Build)

## 0. Document Status

This is the final, consolidated specification, superseding all prior drafts (v1.0 → v1.1.3.1). It folds in every fix made across the review cycle: cross-platform resource handling, the WASM/WIT extensibility layer, the Zero-Trust Reproducibility Gate, the cold-boot AI escalation model, and the GGUF-based inference pipeline. Treat this document as authoritative. Where this spec and any earlier draft disagree, this one wins.

---

## 1. Core Architectural Philosophy

Sprawl is a native, local-first codebase overseer and storage-triage daemon, governed by five pillars:

| Pillar | Statement | Why it matters |
|---|---|---|
| **Heuristics for the Fast Path** | If a regex, WASM plugin, or declared pattern can solve a problem, it executes instantly at ~0% CPU. | Keeps the daemon's steady-state cost near zero; AI is the exception, not the default. |
| **Cold-Boot AI Escalation** | The LLM is never resident in memory during normal operation. It loads on demand, only when deterministic rules fail *and* the user explicitly authorizes the task. | Makes the "negligible idle footprint" promise literally true, not just directionally true. |
| **Zero-Trust Delegation** | WASM plugins may *advise* on anything, including reproducibility and stack identity, but irreversible actions are independently re-verified by hardcoded core invariants before they're permitted. | A buggy or malicious plugin can mislead the UI, but cannot cause data loss. |
| **Declarative Extensibility** | Framework knowledge, sweep targets, and reproducibility logic live in dynamic WASM plugins and layered config files — never hardcoded into the compiled binary. | New ecosystems are added by writing a plugin, not shipping a new Sprawl release. |
| **Absolute Consent** | No permanent deletion, no edits to version-controlled directories, no script execution, no network verification — without explicit terminal consent at the moment of action. | The product's entire trust proposition rests on never surprising the user. |

---

## 2. System Architecture & Tech Stack

Everything is written in Rust for memory safety and single-binary cross-platform distribution. Each dependency below was chosen after rejecting at least one alternative for a specific, documented reason — these rejections are settled; see the companion handover document for the "do not re-litigate" list.

| Layer | Choice | Rejected alternative | Why rejected |
|---|---|---|---|
| Core daemon/CLI | Rust (`cargo`) | — | Memory safety, no runtime, true single-binary cross-compilation. |
| Filesystem watcher | `notify` crate | Raw `inotify` / manual polling | `notify` abstracts `inotify` (Linux), `FSEvents` (macOS), `ReadDirectoryChangesW` (Windows) behind one API. |
| Resource throttling | OS scheduling priority (`nice`/`IDLE_PRIORITY_CLASS`) + `sysinfo` RAM polling | `cgroups` + `systemd` hard limits | `cgroups`/`systemd` are Linux-only; there is no macOS/Windows equivalent, which would have made the safety guarantee platform-dependent. |
| Local ledger | `rusqlite`, unencrypted at the file level | `sqlcipher` (full-database encryption) | SQLCipher requires linking OpenSSL/equivalent per-platform — a real cross-compilation burden for marginal benefit over field-level encryption. |
| Sensitive-field encryption | `aead` crate (AES-GCM), values encrypted before insertion; master key in OS keyring | — | Same practical protection against casual disk reads, without the build complexity above. |
| Vector store / RAG | `lancedb` | `qdrant` | Qdrant's Rust client talks to a server process over gRPC — there's no embedded mode in the Rust client (only in the Python client). That reintroduces "run a separate service," which `lancedb` (file-based, embedded, like SQLite) avoids entirely. |
| Plugin host | `wasmtime`, WASM Component Model (WIT) | Native dynamic library loading (`.so`/`.dll`) | True memory-safe sandboxing (no filesystem/network access beyond what's explicitly granted) and a typed cross-language ABI, without native ABI-stability problems. |
| Inference engine | `candle` (pure-Rust ML framework), reading `.gguf` via `candle_core::quantized::gguf_file` | `llama.cpp` C++ bindings; `.safetensors`-only quantized weights | `candle`'s GGUF support is native Rust — no FFI required either way. GGUF is also the format the quantized-model ecosystem actually ships in; restricting to safetensors would have made the reference model effectively undownloadable. |
| Secret storage | `keyring` crate | Custom encrypted file | Delegates to the OS's own credential store (Secret Service / Keychain / Credential Manager) — stronger guarantee than anything Sprawl could implement itself. |
| Terminal UI | `ratatui` + `crossterm` | Python + Textual | Matches the "near-zero footprint" brand; a Python TUI would undercut that pitch with interpreter startup cost. |
| Clipboard | `arboard`, with a 60-second detached-fork fallback on Wayland | `arboard` alone | Several Wayland compositors expect the copying process to stay alive to serve paste requests; a one-shot CLI process exiting immediately can leave the clipboard empty. The detached helper has a hard TTL so it can't linger indefinitely. |

**Target binary footprint:** <45MB, accounting for the WASM runtime and inference engine. Contains zero model weights.

---

## 3. The Inference Engine & Payload Lifecycle

The executable and the LLM payload are deliberately separated so the binary stays small and the AI escalation tier remains optional infrastructure, not a baked-in dependency.

**The binary** ships with the CLI, TUI, WASM host, and inference *engine* — but no model weights.

**First-run download.** The first time any AI-escalated command runs (e.g. `sprawl analyze --deep`), Sprawl prompts the user to download a quantized model (e.g. `Phi-3-mini-4k-instruct-q4.gguf`).

**Verification & resilience:**
- The download URL and expected SHA-256 checksum are hardcoded into the binary at build time.
- The downloaded blob is hashed in memory before it touches disk in its final location.
- A checksum mismatch purges the file and aborts the cold-boot — no partial or unverified weights are ever loaded.
- A cryptographically verified fallback mirror is included for resilience against a dead primary URL (DNS failure, non-200 response, etc.).

**Caching.** Models live at `~/.sprawl/models/*.gguf`. Sprawl reads flat GGUF files directly and does not attempt to parse external tool caches (e.g. Ollama's blob-store manifest format) — this trades potential disk-space sharing for implementation stability.

**Cold-boot UX.** Loading multi-gigabyte weights from disk is synchronous and drive-speed-dependent. The UI does not block: pressing `[W]` to authorize an LLM task transitions the status bar to a pulsing `[WAKING INFERENCE CORE...]` with an I/O progress indicator, so the wait is legible rather than feeling like a hang.

**Pre-flight RAM check** (see §5D) gates every cold-boot before it starts.

---

## 4. The Extensibility Engine

### 4.A WASM `StackDetector` Plugins (WIT ABI)

- Sprawl adopts the **WASM Component Model (WIT)** for plugin interfaces — "compiles to WASM" alone isn't an ABI; WIT gives a strongly typed contract that Rust, Go, and C implementations can all satisfy identically.
- A single plugin identifies the stack, extracts dependencies, **and advises** on reproducibility (lockfile presence, local patches) — but see §5C: that advice is never trusted blindly for destructive actions.
- Sprawl ships **cryptographically signed, first-party** `.wit` components for Node, Rust, Python, and Go.
- **Community plugins** are installed explicitly via `sprawl plugin install <path/URL>`, run fully sandboxed (no filesystem access beyond declared scope, no network), and are treated as **advisory only**.
- *Known open item:* community plugin downloads do not yet have a checksum/signature verification step at install time (unlike the model payload in §3). Risk is bounded by the Zero-Trust invariant in §5C, but this should be closed before GA.

### 4.B Hierarchical Configuration & Merge Semantics

Configuration resolves through a cascading hierarchy:

```
Global Defaults → Persona Preset → Team Template (checked-in repo config) → Project Override
```

- **Arrays merge by concatenation.** If Global excludes `["node_modules"]` and Project excludes `[".venv"]`, the effective exclusion list is `["node_modules", ".venv"]`.
- **Boolean toggles** (e.g. `nuke_safe`) are overridden by the most specific layer present.
- **Predicates are full-replacement, not ANDed.** If a sweep target defines `condition = "idle_days > 30"` at one layer and a more specific layer redefines the same target's condition, the more specific layer's condition string completely replaces the upstream one. Predicates are explicitly treated as **local defaults, not immutable global policy** — Team Template sets a sensible starting point, not an enforced floor.
- **Visual Override Trace.** Because silent predicate replacement could otherwise hide a real change in triage behavior, the `sprawl ui --triage` view shows a non-blocking indicator — `[⚠️ Config Overridden by Project Local]` — next to any item whose condition was overridden by a more specific layer.
- **Noise patterns.** `patterns.toml` supports a `noise_patterns` list so users can suppress domain-specific high-entropy false positives (Webpack chunk hashes, bcrypt test fixtures) from the Sentinel Inbox without disabling entropy scanning globally.

---

## 5. The Escalation Sub-Systems

Every core engine follows the same shape: cheap deterministic pass first, human-authorized AI escalation only when the deterministic pass is insufficient.

### A. The Archaeologist (Project Mapping)

1. **Layer 1 — WASM Fast Path.** Iterates loaded `StackDetector.wasm` plugins to parse known manifests and deduce frameworks deterministically. Instant, ~0% CPU.
2. **Layer 2 — Drift Alert.** Once 5 *unique project roots* (deduplicated, not raw events) accumulate an "Unknown Stack" flag, the TUI surfaces a passive suggestion to re-run the persona profiler.
3. **Layer 3 — HITL Deep Analyze.** `sprawl analyze --deep` cold-boots the LLM to reverse-engineer custom build logic for a flagged project. Before writing anything to disk, it explicitly prompts `[L]ocal / [g]lobal` to decide whether the derived `.sprawl.toml` is written into the project root or kept in Sprawl's own cache — never an unconsented write into a (possibly version-controlled) project directory.

### B. The Token Sentinel (Secret Security)

1. **Layer 1 — Fast Path + Negative Filter.** Shannon-entropy scanning flags high-randomness strings, immediately discarding UUIDs, git SHAs, JWTs, and other known-shape non-secrets via regex, combined with user-defined `noise_patterns`. Matches against known provider-prefix regexes (`sk_live_...` etc.) are hashed, AES-encrypted, and vaulted to the OS keyring with **zero network calls**.
2. **Layer 2 — HITL Batch Inbox.** High-entropy strings that survive the negative filter but match no known prefix do **not** wake the LLM automatically — that would violate the cold-boot-only-on-explicit-authorization rule. They queue in the TUI Sentinel Inbox. The user manually tags them, or presses `[W]` to authorize one cold-booted LLM batch-classification pass over the whole queue at once.
3. **Layer 3 — Network Verify.** Strictly opt-in. `sprawl verify --key [ID]` delegates the live provider ping to a read-only, user-installed MCP server — the core daemon itself never makes an outbound call.

### C. The Sweeper (Storage Triage)

- **Targets** are driven entirely by `patterns.toml`, resolved through the hierarchy in §4.B — nothing about what counts as "sweepable" is hardcoded.
- **The Reproducibility Safety Gate — Zero-Trust Invariant.** This is the single most important safety mechanism in the system:
  1. The relevant WASM plugin returns `is_reproducible = true` (or `false`) as advice.
  2. The Rust core **independently re-verifies this itself** — scanning for the declared lockfile, `.diff`/`patch-package` artifacts, and `file://`/`link:` local dependency paths.
  3. If the plugin says reproducible but the core's own scan finds a local override the plugin missed (or lied about), **the core's veto wins**: the `[X] Nuke` action is permanently disabled in the UI for that target, full stop.
  - This means a buggy, outdated, or actively malicious community plugin can never be the sole authority that grants a destructive, irreversible action.
- **The Atomic Move.** Approved "Archive" actions move heavy, reproducible-but-large assets to a secondary drive **uncompressed**, leaving a native OS symlink in place so dependent scripts keep working transparently.
- **The Manifest.** Every Sweeper action — Nuke or Archive — is appended to `~/.sprawl/manifest.json`, an append-only log that makes `sprawl restore [PROJECT_NAME]` a deterministic, exact reversal.

### D. The Archivist (Resource Yielding & Semantic Search)

Two distinct resource-safety mechanisms, deliberately separated because they protect against different failure modes:

- **Foreground HITL pre-flight check.** Before any user-commanded cold-boot (e.g. pressing `[W]`), Sprawl checks absolute memory headroom:
  `Available RAM > (Model Size in RAM + 1024MB Safety Margin)`
  If insufficient, it aborts loudly and specifically — e.g. `[!] INSUFFICIENT HEADROOM: 3.1GB required, 2.8GB available. Aborting cold-boot.` — rather than starting a load that risks an OOM mid-inference.
- **Background embedding thread.** The semantic-indexing thread (a small embedding model, not the full LLM) runs at the lowest OS-available scheduling priority and continuously polls RAM via `sysinfo`, suspending and resuming itself as memory pressure rises and falls — this one *can* be safely paused mid-work, unlike a foreground inference pass.
- **Vector storage:** `lancedb`, fully embedded and file-based — no server process, no gRPC.

---

## 6. Security & Isolation Model

- **Application-layer encryption.** Sensitive ledger columns (hashed key references, cloud IDs) are AES-GCM encrypted before insertion; the SQLite file itself is not whole-database encrypted (see §2 rationale).
- **No plaintext secrets.** Discovered credentials are wiped from Sprawl's own memory immediately after being handed to the OS keyring.
- **Air-gapped core operations.** Routine daemon operation makes zero outbound network calls. The only exceptions — the first-run model download (§3) and opt-in MCP-routed cloud verification (§5B Layer 3) — are explicitly consent-gated and documented, not silent.
- **Strict sandboxed execution.** `sprawl resurrect` never auto-executes a generated recovery script natively. If Docker/Podman isn't installed, it outputs the script as text, warns the user explicitly, and hard-exits. There is no "run unsandboxed anyway" override — the risk of a hallucinated destructive command in an LLM-reconstructed script for an undocumented project is too asymmetric to allow a forced path.
- **Plugin trust boundary.** WASM sandboxing (via `wasmtime`) protects the host machine from a plugin doing anything outside its declared capabilities. The Zero-Trust Reproducibility Gate (§5C) is what protects the user's *data* from a plugin that returns a wrong-but-sandboxed-legal verdict. These are two different protections and both are necessary.

---

## 7. The Terminal User Interface (TUI)

Built entirely in `ratatui`. Immediate-mode rendering redraws only on a keystroke or a backend daemon event — genuinely 0% CPU while sitting idle on a monitor.

| Tab | Flag | Purpose |
|---|---|---|
| 1. Dashboard | `--home` | Global health metrics, storage distribution, active/dormant ratios, ecosystem drift alerts. |
| 2. Sweeper Inbox | `--triage` | Per-directory triage actions: `[X] Nuke`, `[A] Archive`, `[S] Snooze`. `[X]` is dynamically disabled when the Safety Gate vetoes it; overridden predicates show the `[⚠️ Config Overridden by Project Local]` indicator. |
| 3. Sentinel Inbox | `--sentinel` | Review queued ambiguous high-entropy strings; manual tag or `[W]` to authorize a batch LLM pass. Status bar shows `[WAKING INFERENCE CORE...]` during processing. |
| 4. Semantic Search | `--search` | Embedded fuzzy-finder over the local `lancedb` RAG index for instant cross-project code retrieval. |

---

## 8. CLI Command Surface ("The Active Commander")

| Command | Purpose | Safety notes |
|---|---|---|
| `sprawl daemon start\|stop\|status` | Manage the low-priority background watcher. | — |
| `sprawl ui` | Launch the interactive Ratatui dashboard. | — |
| `sprawl plugin install <path/URL>` | Install a third-party WASM `StackDetector` component. | Runs sandboxed; advisory only; verdicts subject to §5C veto. |
| `sprawl plugin list\|remove\|update` | Manage installed plugin inventory. | — |
| `sprawl profile-machine` | Run WASM detectors machine-wide first; prompts for consent before falling back to an LLM scan of unrecognized directories. | Consent-gated before any AI step. |
| `sprawl analyze [DIR] --deep` | Cold-boot the LLM to map an undocumented/custom-build repository. | Prompts `[L]ocal/[g]lobal` before any disk write. |
| `sprawl bundle [DIR]` | Package a project into a token-optimized, markdown-fenced (`<file path="...">`) context schema via deterministic AST stripping. | No AI involved — pure heuristic pass. |
| `sprawl resurrect [DIR] --export-agent` | Compile codebase bundle + framework diagnostics into `resurrection-kit.md`; copy to clipboard via `arboard` (Wayland: detached helper, 60s TTL). | No local script execution in this mode. |
| `sprawl verify --key [ID]` | Opt-in live key verification via a user-installed MCP server. | Core daemon never calls out directly. |
| `sprawl simulate-revoke [KEY]` | Compute the Local Blast Radius — which `.env` files and Docker states break if this key dies — via the `lancedb` index. | Read-only, local-only. |
| `sprawl restore [PROJECT_NAME]` | Reverse a Sweeper action using the append-only manifest. | Deterministic; exact inverse of the original Archive/Nuke entry. |

---

## 9. Filesystem & State Layout

```
~/.sprawl/
├── ledger.db              # SQLite, app-layer AES-encrypted sensitive columns
├── manifest.json           # append-only Sweeper action log (Archive/Nuke/restore)
├── patterns.toml           # Global Defaults + Persona Preset layer
├── models/
│   └── phi-3-mini-4k-instruct-q4.gguf
├── plugins/
│   └── <plugin-name>.wasm  # community + first-party StackDetector components
└── cache/
    └── <project-id>.sprawl.toml   # global-scope derived configs (declined local write)

<project-root>/
├── .sprawl.toml             # Project Override layer (opt-in local write)
└── sprawl.team.toml         # Team Template layer (checked into repo, shared via VCS)
```

Team Template config is distributed exactly the way the rest of a repo is — committed to version control — which is why it doesn't conflict with the air-gapped-core principle in §6: Sprawl never fetches it itself.

---

## 10. Known Limitations & Deferred Items (v1.2)

- **Community plugin install has no checksum/signature step** (§4.A). Bounded by the Zero-Trust invariant; recommended fix before GA.
- **Default model license/redistribution terms** for the shipped reference model (e.g. Phi-3) have not yet been formally reviewed for this auto-download flow.
- **No GUI/non-terminal frontend.** Intentionally TUI/CLI-only for v1.2.
- **No cross-machine sync.** Config, ledger, and manifest are all local to a single machine by design — Team Template is the only intentionally shared layer, and it travels via the user's own VCS, not a Sprawl-run sync service.

---

## 11. Versioning & Compatibility Policy

- The model download URL + SHA-256 are pinned **per binary release**. Shipping a new default model requires a new Sprawl release — there is no independent model-update channel.
- The WIT plugin ABI is versioned; Sprawl rejects a plugin on a major-version ABI mismatch rather than risk undefined behavior from a stale interface.
- `patterns.toml`/`.sprawl.toml` schema changes are versioned; the daemon backs up `ledger.db` before applying any schema migration on startup.
