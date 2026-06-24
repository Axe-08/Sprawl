# Sprawl — Project Handover & Context Document

## 1. Purpose of This Document

This document exists so an incoming agent, contributor, or collaborator can pick up the Sprawl project at full context **without re-reading the entire design conversation**. It captures *why* the architecture looks the way it does, which alternatives were considered and rejected, and what's genuinely still open versus settled. The authoritative technical spec is `SPRAWL_DESIGN_SPEC_v1.2.md` — this document is the narrative and decision trail behind it.

---

## 2. Project Summary

Sprawl is a native, local-first background daemon and CLI toolkit for developers with a sprawling, messy local dev environment: abandoned projects, scattered API keys, bloated ML datasets, and undocumented repos. It runs almost entirely on deterministic heuristics (regex, AST parsing, WASM plugins) and only escalates to a local LLM as an explicit, user-authorized "escape hatch" for genuinely ambiguous cases — never silently, never resident in memory by default.

---

## 3. Evolution Log (Why It Looks Like This)

The design went through several full revisions. Each row is a real pivot, not cosmetic — useful context for understanding why certain things are built the way they are.

| Version | Core issue surfaced | Resolution adopted |
|---|---|---|
| v1.0 (initial) | Resource-limiting design (`cgroups`, `/proc`, `inotify`) was implicitly Linux-only despite a cross-platform pitch. | Moved to OS-abstracted crates (`notify`, OS scheduling priority) instead of kernel-specific primitives. |
| v1.0 | Live API key verification could trip provider fraud detection or hit non-free endpoints. | Split into a passive (free, local) classification layer and a strictly opt-in, manual network-verify layer routed through MCP. |
| v1.0 | "Resurrect" could hand back a wrong run script with false confidence. | Required sandboxed test execution (Docker) before trusting LLM-generated output. |
| v1.1 | LLM was used even for cheap, deterministic work (stack detection from manifests). | Introduced the **Escalation Path**: heuristics first, AI only as fallback. |
| v1.1 | Idle-detection relied on OS idle-time APIs that don't exist uniformly (notably gaps on Wayland). | Replaced with OS scheduling-priority yielding (`nice`/equivalent) — the OS naturally starves the background thread under real load, no custom polling needed. |
| v1.1 | Qdrant's Rust client requires a running server process, contradicting the single-binary pitch. | Replaced with `lancedb` (genuinely embedded, file-based). |
| v1.1.1 | `StackDetector` logic was still compiled into the core binary — same rigidity problem the cloud-provider MCP integrations had already solved. | Moved to a WASM plugin architecture so new ecosystem support doesn't require a Sprawl release. |
| v1.1.1 | Token Sentinel's "ambiguous string" escalation woke the LLM automatically mid-scan — violated the "AI only on explicit request" principle, and was noisy (UUIDs/SHAs/JWTs are high-entropy too). | Added a negative filter (drop known-shape non-secrets first) and moved escalation to a manual, batched, user-authorized Inbox review — never automatic. |
| v1.1.2 | The reproducibility check that gated destructive deletes lived only inside a community-WASM-plugin verdict — i.e., third-party code could grant an irreversible action. | Introduced the **Zero-Trust Reproducibility Gate**: the Rust core independently re-verifies lockfile/patch/local-path presence itself and can veto the plugin. This is the single most important safety invariant in the system — do not weaken it. |
| v1.1.2 | "Compiles to WASM" wasn't an actual ABI for cross-language plugins. | Adopted the WASM Component Model (WIT) for a real typed interface. |
| v1.1.2 | RAM pre-flight check used a flat percentage threshold, which is wrong at both ends of the hardware spectrum. | Changed to an absolute formula: `available RAM > model size + fixed margin`. |
| v1.1.3 | Inference engine was left as "candle or llama.cpp bindings" — an unresolved FFI/build-complexity question. | Committed to `candle` only, citing `.safetensors` to avoid C++ FFI. |
| v1.1.3 → v1.1.3.1 | The safetensors-only restriction was based on a **false premise**: `candle`'s GGUF support is pure Rust (no FFI either way), and quantized models are almost universally distributed as GGUF, not safetensors — the named reference model likely didn't exist in the format specified. | Switched to native GGUF support via `candle_core::quantized::gguf_file`. This is now verified against current `candle` documentation — confirmed correct. |
| v1.1.3.1 | Predicate config override (Project beating Team Template) could silently erode an org-level safety threshold with no visibility. | Added the Visual Override Trace UI indicator, and explicitly defined Team Template predicates as defaults, not enforced policy. |

---

## 4. Settled Decisions — Do Not Re-litigate

If a future contributor (human or agent) suggests any of the following, point them at the reason it was already rejected:

- **Don't suggest `cgroups`/`systemd` for resource limiting.** Linux-only; breaks the cross-platform safety guarantee.
- **Don't suggest `qdrant` for the vector store.** The Rust client requires a separate server process (confirmed via current docs — no embedded mode outside the Python client). `lancedb` is the embedded equivalent.
- **Don't suggest `sqlcipher` for ledger encryption.** Cross-platform OpenSSL linking is a real build burden; app-layer AES via `aead` gives equivalent practical protection for this threat model.
- **Don't suggest `llama.cpp` C++ bindings, or safetensors-only quantized weights.** `candle` supports GGUF natively in pure Rust — no FFI tradeoff exists either way, and GGUF is what the ecosystem actually distributes.
- **Don't let the LLM auto-trigger on ambiguous secret-scan results.** It must always go through the HITL batch inbox — this was an explicit philosophy violation caught and fixed once already.
- **Don't let a WASM plugin's verdict alone gate a destructive action.** The Zero-Trust Reproducibility Gate (Rust core re-verification) must always be the final word, regardless of what any plugin or TOML predicate says.
- **Don't add a "force unsandboxed execution" override to `sprawl resurrect`.** This was considered and explicitly rejected — the risk asymmetry of executing a hallucinated script against an undocumented project isn't worth the convenience.

---

## 5. Current State

The architecture is considered **feature-complete for v1.2** at the design level. See `SPRAWL_DESIGN_SPEC_v1.2.md` for the full, authoritative specification, including the tech stack table with rejected alternatives, the four escalation engines in full detail, the CLI surface, and the filesystem/state layout.

---

## 6. Known Open Items (Carried Forward, Not Yet Resolved)

These are real but low-severity, and explicitly listed in §10 of the design spec too:

1. Community WASM plugin installs (`sprawl plugin install <URL>`) have no checksum/signature verification step, unlike the model payload download. Bounded risk (Zero-Trust Gate already prevents the worst outcome), but should close before GA.
2. The license/redistribution terms of the default shipped model haven't been formally reviewed for an auto-download flow done on the user's behalf.
3. Persona Preset *contents* (what exactly ships in the web-dev / ML / data-eng / mobile presets) haven't been specified in detail — only the mechanism for them exists.

---

## 7. Recommended Documentation Set for the Build Phase

Beyond the design spec itself, these documents should exist before or shortly after build start — each tied to a specific risk this project carries:

| Document | Why Sprawl specifically needs it |
|---|---|
| **Architecture Decision Records (ADRs)** | One per row in the Evolution Log (§3) above — keeps the *why* attached to the decision permanently, separate from this narrative summary. |
| **Threat model** | The Zero-Trust Gate, plugin sandboxing, and air-gap exceptions deserve a real enumerated trust-boundary document (STRIDE-style or equivalent), not just spec prose. |
| **WIT plugin ABI spec + versioning policy** | Third-party plugin authors need a stable, standalone contract — they shouldn't have to read the whole design spec to write a plugin. |
| **Config schema reference** (`patterns.toml` / `.sprawl.toml`) | The merge semantics (array concat vs. boolean override vs. predicate full-replacement) are subtle enough to drift from the implementation without a versioned, explicit schema doc. |
| **Test plan & acceptance criteria** | Especially a concrete filesystem-state test matrix for the Reproducibility Safety Gate — this is the one component where a test gap directly causes user data loss. |
| **Release & migration policy** | The model hash is pinned per binary release, and the ledger schema will evolve. Both need a stated upgrade path before real users have state to break. |
| **Plugin author guide** | Separate from the WIT ABI spec — a practical "how to write and ship a StackDetector plugin" walkthrough. |
| **Operational runbook** | What a user does if `restore` fails partway, or a migration corrupts `manifest.json` — destructive-adjacent operations need a documented recovery path, not just code that's hopefully correct. |
| **Privacy/telemetry policy** | Should be explicit even if the answer is "Sprawl collects nothing" — trust is the entire value proposition here. |

---

## 8. Suggested Next Steps

1. Write the formal `.wit` interface definitions for the `StackDetector` contract — this unblocks both the first-party plugins and any community plugin work.
2. Scaffold the four first-party signed plugins (Node, Rust, Python, Go) against that interface.
3. Stand up a 3-OS CI matrix (Linux/macOS/Windows) early — most of the resolved issues in this project were cross-platform footguns, and CI is the cheapest way to keep them from coming back.
4. Build the Reproducibility Safety Gate and its test matrix before anything else destructive — it's the component every other safety property depends on.
5. Defer the AI escalation paths (Archaeologist Layer 3, Sentinel Layer 2/3) until the fast-path engines are solid; they're explicitly the "5% case," not the critical path to a usable v1.
