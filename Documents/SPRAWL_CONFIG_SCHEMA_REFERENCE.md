# Sprawl — Configuration Schema Reference

## 1. Files Covered

| File | Layer(s) | Typical location |
|---|---|---|
| `patterns.toml` | Global Defaults, Persona Preset | `~/.sprawl/patterns.toml` |
| `sprawl.team.toml` | Team Template | Checked into the project's own repo, distributed via VCS |
| `.sprawl.toml` | Project Override (manual or LLM-derived) | Project root, or `~/.sprawl/cache/` if the user declined a local write |

All three files share the same schema shape; only their position in the merge hierarchy differs (see §4).

## 2. `patterns.toml` Schema

```toml
schema_version = 1
persona = "ml-engineer"   # optional label, informational + drift-alert context

[[sweep_target]]
path = "node_modules"
condition = "idle_days > 30 AND lockfile_present == true"
action = "nuke_safe"        # nuke_safe | archive | snooze_default

[[sweep_target]]
path = "*.safetensors"
condition = "idle_days > 60"
action = "archive"

[[noise_pattern]]
match = "regex:^[0-9a-f]{8}\\.chunk\\.js$"   # e.g. Webpack chunk hashes
reason = "webpack build artifact"

[[noise_pattern]]
match = "regex:^\\$2[aby]\\$"                # bcrypt hash prefix
reason = "bcrypt fixture"
```

### Field reference

| Field | Type | Required | Notes |
|---|---|---|---|
| `schema_version` | integer | Yes | Used for migration detection (see Release & Migration Policy). |
| `persona` | string | No | Informational label; drives which preset defaults a fresh install starts from. |
| `sweep_target.path` | string (glob) | Yes | Pattern matched against directory/file names. |
| `sweep_target.condition` | string (predicate expression) | No | If omitted, the target is always eligible when matched. See §3 for grammar. |
| `sweep_target.action` | enum | Yes | `nuke_safe` (subject to the Reproducibility Gate — see Design Spec §5C), `archive`, or `snooze_default`. |
| `noise_pattern.match` | string | Yes | `regex:` prefix required; plain substrings are not currently supported to avoid ambiguous partial matches. |
| `noise_pattern.reason` | string | No | Shown in the Sentinel Inbox for transparency when a string is auto-filtered. |

## 3. Predicate Expression Grammar (informal)

Supported operands: `idle_days`, `lockfile_present`, `port_bound`, `git_ahead_count`. Supported operators: `>`, `<`, `>=`, `<=`, `==`, `AND`. `OR` and parenthesized grouping are intentionally **not** supported in v1.2 — kept simple to avoid ambiguous-predicate logic traps (see ADR-013's rationale for full-replacement rather than AND-merging across layers).

```
condition = "idle_days > 30 AND lockfile_present == true"
```

## 4. Merge Algorithm (Formal Restatement)

Given layers in order `Global → Persona → Team → Project`:

1. **`sweep_target` arrays concatenate** across all layers present. A target with the same `path` defined at multiple layers is treated as the *same logical target*; its `condition` and `action` follow rule 2 below, while its presence in the merged list is not duplicated.
2. **For a `sweep_target` defined at multiple layers:** `action` (boolean-like enum) and `condition` (predicate string) are both taken from the **most specific layer that defines them** — i.e., full replacement, not merged logic. This is true even if a less specific layer's `condition` was stricter.
3. **`noise_pattern` arrays concatenate** across all layers with no deduplication logic beyond exact string match — redundant patterns are harmless, just slightly wasteful.
4. **Whenever rule 2 causes an effective override** (a more specific layer changed `condition` or `action` for a target also defined upstream), the UI must surface this — see UX Guide §3, the `[⚠️ Config Overridden by Project Local]` indicator.

## 5. `.sprawl.toml` Schema (Project-Level Derived Config)

```toml
schema_version = 1
stack = "go"
entry_point = "cmd/server/main.go"
build_command = "go build ./..."
run_command = "go run ./cmd/server"
reproducible = true
source = "llm-derived"        # "manual" | "llm-derived" | "plugin-detected"
derived_at = "2026-06-20T00:00:00Z"
```

| Field | Notes |
|---|---|
| `source` | Tracks provenance — distinguishes a human-written override from an `analyze --deep` result, relevant for trust/review purposes. |
| `reproducible` | This field is **informational only** in this file; it is never read as authoritative by the Sweeper. The Sweeper always re-derives reproducibility itself per ADR-009, regardless of what's recorded here. |

## 6. Schema Versioning

- Every config file declares `schema_version`.
- A daemon encountering a `schema_version` older than its current supported version runs an automatic, logged migration (see Release & Migration Policy for the backup-before-migrate process).
- A `schema_version` newer than the running daemon supports causes the daemon to refuse to load that file and prompt the user to upgrade Sprawl, rather than guessing at an unknown schema shape.
