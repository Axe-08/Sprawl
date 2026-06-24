# Sprawl — Architecture Decision Record (ADR) Log

Each entry follows: Status / Context / Decision / Consequences. These are settled decisions — see §6 for the explicit "do not re-litigate without new evidence" guidance.

---

### ADR-001: Cross-Platform Resource Management
**Status:** Accepted

**Context:** Initial design relied on `cgroups`, `systemd` user services, and `/proc/loadavg` for hard resource ceilings and load detection.

**Decision:** Replaced with OS-abstracted crates (`notify` for filesystem events, OS-native scheduling priority via `nice`/`IDLE_PRIORITY_CLASS`, `sysinfo` for RAM/CPU polling). No kernel-specific primitive is load-bearing for a safety guarantee.

**Consequences:** The daemon's "cannot freeze your machine" promise now holds identically on Linux, macOS, and Windows, at the cost of losing `cgroups`' hard kernel-enforced ceiling in favor of cooperative OS scheduling.

---

### ADR-002: API Key Network Verification Policy
**Status:** Accepted

**Context:** Automated live verification of every discovered key risks billable side effects and provider fraud-detection flags.

**Decision:** Split into a free, local, passive classification layer (entropy + prefix regex) and a strictly opt-in, manual, MCP-routed verification layer (`sprawl verify --key`).

**Consequences:** No key is ever pinged without an explicit per-key user action; verification coverage depends on the user actually running the opt-in command.

---

### ADR-003: Sandboxed Execution Requirement for `resurrect`
**Status:** Accepted

**Context:** LLM-generated run scripts for undocumented, possibly long-abandoned projects carry real risk of hallucinated destructive commands.

**Decision:** `resurrect` never auto-executes a generated script outside Docker/Podman. If no sandbox is available, the script is output as text and the command hard-exits. No "force unsandboxed" override exists.

**Consequences:** Slightly less convenient on machines without a container runtime, in exchange for eliminating an entire class of irreversible-mistake risk.

---

### ADR-004: Escalation Path Architecture
**Status:** Accepted

**Context:** Early designs invoked AI for tasks (e.g., manifest-based stack detection) that deterministic parsing already solves cheaply and reliably.

**Decision:** Every subsystem follows a tiered model — heuristics/WASM first, AI only as an explicit, user-authorized fallback for genuinely ambiguous cases. AI is never resident in memory during normal operation.

**Consequences:** Keeps steady-state resource cost near zero and makes AI behavior fully predictable and auditable, at the cost of requiring every subsystem to be designed with an explicit two-tier structure rather than a single AI-driven implementation.

---

### ADR-005: Resource Yielding via OS Scheduling Priority
**Status:** Accepted

**Context:** OS idle-time detection APIs (used to decide when it's safe to run background indexing) are not uniformly available — notably gaps on several Wayland compositors lacking the idle-notify protocol.

**Decision:** Background indexing threads run at the lowest OS-available scheduling priority and rely on natural OS starvation under real load, rather than polling for an explicit "idle" signal.

**Consequences:** Removes a whole category of platform-availability risk; background work may start slightly more eagerly than a strict idle-only policy would allow, mitigated by the RAM pre-flight/suspend logic in ADR-011.

---

### ADR-006: Embedded Vector Store
**Status:** Accepted

**Context:** `qdrant`'s Rust client connects over gRPC to a running Qdrant server process — confirmed via current documentation, no embedded mode exists outside the Python client.

**Decision:** Use `lancedb`, which is genuinely embedded and file-based.

**Consequences:** Preserves the single-binary distribution model with no extra service to manage; ties the project to `lancedb`'s feature set and maturity curve.

---

### ADR-007: WASM Plugin Architecture for Stack Detection
**Status:** Accepted

**Context:** Hardcoding ecosystem-specific manifest parsers into the core binary meant every new stack required a Sprawl release — the same rigidity already solved for cloud-provider integrations via MCP.

**Decision:** Stack detection moves to dynamically-loaded WASM plugins behind a typed WIT interface (see ADR-010), sandboxed via `wasmtime`.

**Consequences:** New ecosystem support no longer blocks on a core release; introduces a plugin trust question, resolved separately by ADR-009.

---

### ADR-008: HITL Batch Inbox for Ambiguous Secret Classification
**Status:** Accepted

**Context:** An earlier design had the Token Sentinel wake the LLM automatically whenever a high-entropy string lacked a recognizable provider prefix — violating the "AI only on explicit request" principle (ADR-004) and triggering frequently on UUIDs, SHAs, and JWTs.

**Decision:** Ambiguous strings are filtered through a negative-match list first, then queued in a TUI inbox for manual tagging or a single user-authorized batch LLM pass — never an automatic per-event escalation.

**Consequences:** Eliminates unpredictable cold-boots during routine scanning; requires the user to periodically clear the inbox rather than getting instant automatic classification.

---

### ADR-009: Zero-Trust Reproducibility Gate
**Status:** Accepted — highest-priority invariant in the system

**Context:** Once stack/reproducibility detection moved to community-extensible WASM plugins (ADR-007), a destructive action (`Nuke`) could end up gated entirely by third-party code.

**Decision:** The Rust core independently re-verifies reproducibility (lockfile presence, absence of local patches/`file://` paths) on every Nuke-eligibility check. A plugin's affirmative verdict is advisory only; the core's own scan can veto it, and that veto is final.

**Consequences:** No plugin, however buggy or malicious, can ever single-handedly authorize an irreversible action. This is the load-bearing safety property for the entire plugin extensibility model — do not weaken it to "trust the plugin" for performance or simplicity reasons.

---

### ADR-010: WIT Component Model for Plugin ABI
**Status:** Accepted

**Context:** "Compiles to WASM" is not itself an ABI — cross-language plugins (Rust/Go/C) need a real typed contract.

**Decision:** Adopt the WASM Component Model (WIT) for the `StackDetector` interface.

**Consequences:** Plugin authors get a stable, documented, cross-language interface; Sprawl takes on the dependency and complexity of `wasmtime`'s component-model tooling.

---

### ADR-011: Absolute-Value RAM Pre-Flight Check
**Status:** Accepted

**Context:** A flat percentage-of-total-RAM threshold is wrong at both extremes of the hardware spectrum (too conservative on large-RAM machines, potentially insufficient on small-RAM machines).

**Decision:** Pre-flight check compares available RAM directly against (model size + fixed safety margin) in absolute terms.

**Consequences:** Correctly scales across hardware; requires the system to know the loaded model's actual RAM footprint ahead of time.

---

### ADR-012: Inference Engine & Model Format Selection
**Status:** Accepted

**Context:** An earlier draft restricted models to `.safetensors` specifically to avoid C++ FFI complexity from `llama.cpp` bindings — but `candle`'s GGUF support (`candle_core::quantized::gguf_file`) is implemented natively in Rust, so no FFI tradeoff existed either way. Meanwhile, quantized models are distributed almost universally as GGUF, not safetensors.

**Decision:** Use `candle` exclusively, reading `.gguf` weights natively.

**Consequences:** Matches the actual quantized-model distribution ecosystem and avoids a real risk that the reference model in the spec wouldn't exist in the format previously specified.

---

### ADR-013: Predicate Override Semantics
**Status:** Accepted

**Context:** Allowing a more specific config layer (Project) to fully override a less specific layer's (Team Template) sweep condition risked silently eroding an org-level safety threshold with no visibility.

**Decision:** Predicates are explicitly treated as local defaults rather than enforced policy — full replacement by the most specific layer is allowed, but the TUI surfaces a `[⚠️ Config Overridden by Project Local]` indicator whenever this happens.

**Consequences:** Keeps the merge model simple (no conflicting-predicate-AND logic to reason about) while ensuring the override is never silent.
