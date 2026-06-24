# Sprawl — Technical Requirements Document (TRD)

## 1. Purpose & Relationship to Other Documents

This document states *what the system must satisfy*, as testable requirements, independent of implementation detail. The Design Spec (`SPRAWL_DESIGN_SPEC_v1.2.md`) states *how* those requirements are satisfied. The Test Plan traces back to the requirement IDs below, not to the architecture document — requirements should outlive any single implementation choice.

---

## 2. Functional Requirements

| ID | Requirement |
|---|---|
| FR-1 | The system SHALL detect project stack/framework for at least Node.js, Rust, Python, and Go via deterministic parsing, without invoking AI inference. |
| FR-2 | The system SHALL NOT permit a permanent-delete ("Nuke") action on any directory unless reproducibility has been independently verified by the core engine itself — a plugin's or predicate's affirmative verdict alone is insufficient. |
| FR-3 | The system SHALL log every Sweeper action (Archive or Nuke) to an append-only manifest sufficient to deterministically reverse an Archive action. |
| FR-4 | The system SHALL classify discovered high-entropy strings into "known provider," "filtered noise," or "ambiguous — needs review" without making any network call during classification. |
| FR-5 | The system SHALL NOT invoke local LLM inference automatically as a side effect of a passive scan; LLM inference SHALL only begin following an explicit, separate user authorization for that specific task. |
| FR-6 | The system SHALL NOT make any outbound network request without a per-action, explicit, documented consent point (model download, MCP-routed verification). |
| FR-7 | The system SHALL support loading third-party stack-detection plugins at runtime without requiring a recompiled binary. |
| FR-8 | The system SHALL provide a command to reverse any Sweeper action previously recorded in the manifest. |
| FR-9 | The system SHALL refuse to auto-execute an AI-generated recovery script outside of an isolated sandbox; no unsandboxed execution path SHALL exist. |
| FR-10 | The system SHALL resolve configuration through a defined, deterministic merge order (Global → Persona → Team → Project) with no ambiguous precedence case. |
| FR-11 | The system SHALL surface, in the UI, any case where a more specific configuration layer has overridden a less specific layer's sweep condition. |

## 3. Non-Functional Requirements

| ID | Category | Requirement | Verification method |
|---|---|---|---|
| NFR-1 | Performance | Background daemon CPU usage SHALL average ~0% over a 10-minute idle window (no filesystem events, no user input) on reference hardware. | Automated benchmark in CI. |
| NFR-2 | Footprint | Shipped binary size SHALL NOT exceed 45MB, excluding downloaded model weights. | CI artifact size check on release build. |
| NFR-3 | Portability | All daemon resource-management and filesystem-watch behavior SHALL behave equivalently on Linux, macOS, and Windows — no feature SHALL silently degrade on a platform without an explicit, documented fallback. | Cross-platform CI matrix; manual verification per release. |
| NFR-4 | Security | All sensitive ledger fields (key hashes, cloud identifiers) SHALL be encrypted at the application layer (AES-GCM) before persistence. | Static review + unit test asserting plaintext never reaches disk. |
| NFR-5 | Security | Discovered raw secret values SHALL be wiped from process memory immediately after being handed to the OS keyring. | Code review checklist item; memory-zeroing unit test where feasible. |
| NFR-6 | Reliability | A failed or corrupted model download SHALL be detected via checksum mismatch before the model is ever loaded for inference. | Integration test: deliberately corrupt a downloaded blob, assert abort. |
| NFR-7 | Reliability | A foreground AI task SHALL refuse to start if available RAM is below (model size + fixed safety margin), rather than risk an OOM mid-inference. | Integration test with simulated low-memory condition via `sysinfo` mock. |
| NFR-8 | Safety | The Reproducibility Safety Gate SHALL independently re-verify lockfile/patch/local-path state on every Nuke-eligibility check, regardless of plugin verdict caching. | See Test Plan §3 (Safety Gate matrix) — mandatory, non-waivable for release. |
| NFR-9 | Extensibility | A new ecosystem stack detector SHALL be addable via a plugin without modifying or recompiling the core binary. | Manual acceptance test: build and load a new plugin against the WIT interface without touching `cargo` core crates. |
| NFR-10 | Usability | Any destructive or irreversible action exposed in the TUI SHALL require a confirmation step proportional to its risk tier (single keypress for reversible Archive; explicit typed confirmation for permanent Nuke). | UX review + manual test per release. |
| NFR-11 | Auditability | Every consent-gated network call (model download, MCP verification) SHALL be logged locally with a timestamp and the specific action that triggered it. | Log inspection test. |

## 4. System Constraints

- The entire system SHALL be implemented in Rust; no Python interpreter or Docker daemon SHALL be a required runtime dependency for core (non-AI, non-resurrect-sandbox) functionality.
- All third-party/community code execution (plugins) SHALL occur inside a WASM sandbox with no ambient filesystem or network access beyond explicitly granted capabilities.
- The system SHALL NOT depend on a continuously-running external server process for any core feature (vector store, plugin host, or otherwise).

## 5. Traceability

Each requirement above carries an ID (`FR-n` / `NFR-n`) intended for direct reference from the Test Plan's test case table, so that test coverage gaps against this document are mechanically identifiable rather than inferred from architecture prose.
