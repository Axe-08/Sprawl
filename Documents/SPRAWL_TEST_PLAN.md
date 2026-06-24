# Sprawl — Test Plan & Acceptance Criteria

## 1. Test Categories

| Category | Scope |
|---|---|
| Unit | Heuristic parsers, entropy calculation, config merge logic, predicate evaluation |
| Integration | Daemon + filesystem watcher behavior, end-to-end Sweeper/restore cycles, model download + checksum flow |
| Safety-critical | Reproducibility Safety Gate — see §3, non-waivable for any release |
| Cross-platform parity | Identical behavior verification across Linux/macOS/Windows |
| Security | WASM plugin sandbox boundary fuzzing, checksum tamper handling, OOM pre-flight accuracy |
| Manual / exploratory | TUI usability, error-message clarity, first-run flow |

## 2. Traceability

Every test case below references the TRD requirement ID(s) it verifies (`FR-n` / `NFR-n`). A requirement with no referencing test case is a coverage gap and blocks release sign-off.

## 3. Reproducibility Safety Gate Test Matrix (Mandatory, Non-Waivable)

This is the single most important test surface in the system — it directly verifies FR-2 and NFR-8.

| # | Filesystem state | Plugin verdict | Expected core behavior |
|---|---|---|---|
| 1 | Lockfile present, no local patches | `is_reproducible = true` | Core agrees; `[X] Nuke` enabled |
| 2 | No lockfile | `is_reproducible = true` | Core vetoes; `[X] Nuke` locked, reason shown |
| 3 | Lockfile present, `patch-package` directory present | `is_reproducible = true` | Core vetoes; `[X] Nuke` locked, reason shown |
| 4 | Lockfile present, `file://` or `link:` dependency present | `is_reproducible = true` | Core vetoes; `[X] Nuke` locked, reason shown |
| 5 | Lockfile present, no local patches | `is_reproducible = false` (plugin overly conservative) | Core's own scan still runs; if core finds no issue, behavior is governed by the more conservative result — `[X] Nuke` locked (plugin's caution is never downgraded by the core) |
| 6 | No plugin available / plugin crashes | N/A | Treated as "unknown reproducibility" — `[X] Nuke` locked by default, never defaults to enabled |
| 7 | Lockfile present, no patches, but `condition` predicate in `patterns.toml` sets `action = "nuke_safe"` regardless of state | `is_reproducible = true` | Confirm the predicate governs *eligibility for the Nuke offer*, not the Gate itself — Gate re-verification in case #2–4 still applies even when a predicate says "nuke this" |

**Acceptance criterion:** all seven rows pass before any release is cut. A regression in row 2, 3, 4, or 6 specifically is treated as a release-blocking security defect, not a standard bug.

## 4. Functional Test Coverage (by TRD ID)

| TRD ID | Test approach |
|---|---|
| FR-1 | Run Archaeologist Layer 1 against fixture repos for Node/Rust/Python/Go with known-correct expected output; assert zero LLM invocations during the run. |
| FR-3 | Execute an Archive action, corrupt nothing, run `restore`; assert byte-identical file tree to pre-Archive state. |
| FR-4 | Feed a fixture set of UUIDs, JWTs, git SHAs, and real-shaped secrets through the Sentinel; assert correct three-way classification with zero network calls observed. |
| FR-5 | Run a full passive scan against a fixture project containing several ambiguous high-entropy strings; assert the LLM process is never spawned without a separate, explicit authorization call. |
| FR-6 | Run the daemon under a network-call-logging harness for a full passive monitoring session with no user-triggered AI/verify actions; assert zero outbound calls logged. |
| FR-9 | Attempt `resurrect` execution with Docker unavailable (mocked); assert hard-exit with no script execution under any flag combination. |
| FR-11 | Set conflicting `condition` values at Team and Project layers for the same target; assert the override indicator renders in the triage view. |

## 5. Non-Functional Test Coverage (by TRD ID)

| TRD ID | Test approach |
|---|---|
| NFR-1 | Automated CI benchmark: idle daemon for 10 minutes on reference hardware, assert average CPU usage stays within tolerance of 0%. |
| NFR-2 | CI build-artifact size check on every release build; fail the pipeline above 45MB (excluding models directory). |
| NFR-3 | Cross-platform CI matrix executes the full integration suite on Linux, macOS, and Windows runners; any platform-specific skip must be explicitly justified and logged, not silent. |
| NFR-6 | Deliberately corrupt a downloaded model blob in a test harness; assert checksum failure is detected before any inference attempt and the file is purged. |
| NFR-7 | Mock `sysinfo` to report low available RAM; assert cold-boot is refused with the specific required/available figures in the error message. |
| NFR-10 | Manual UX review per release: every irreversible action requires the confirmation tier specified in the UX Guide; no action ships with weaker friction than specified. |

## 6. Security Test Coverage

- **WASM sandbox boundary fuzzing:** feed a test plugin malformed/adversarial inputs (oversized paths, symlink loops, attempts to declare dependencies outside the granted scope) and assert no host filesystem access occurs outside the declared project root.
- **Plugin resource exhaustion:** load a deliberately CPU/memory-heavy test plugin; assert `wasmtime` fuel/memory limits terminate it without crashing the daemon.
- **Cargo dependency audit:** `cargo-audit` run as a release-gating CI step; any newly-disclosed advisory in the dependency tree blocks release until triaged.

## 7. Definition of Done — MVP (Phase 1, per PRD §7)

A release is MVP-acceptable only when:
- All seven Reproducibility Safety Gate matrix rows (§3) pass.
- FR-1, FR-2, FR-3, FR-4, FR-6, FR-9 have passing automated coverage.
- NFR-1, NFR-2, NFR-3, NFR-8 have passing automated coverage.
- Manual UX review of the Sweeper Inbox confirmation flow (NFR-10) is signed off.
