# Sprawl — Product Requirements Document (PRD)

## 1. Problem Statement

Developers who work across many local projects accumulate three compounding problems that no single existing tool addresses together:

1. **Storage sprawl** — dormant `node_modules`, ML model weights, and dataset files silently consume disk space across dozens of abandoned or paused projects.
2. **Credential sprawl** — API keys and secrets get scattered across `.env` files and shell configs in projects the developer has long since forgotten, with no map of what's still live or what depends on it.
3. **Context sprawl** — undocumented or half-finished projects become unreadable to their own author after enough time passes, with no record of how to run them or what they were for.

Existing tools solve fragments of this (disk usage analyzers, secret scanners, dependency auditors) but none combine local-first operation, reversibility, and a uniform trust model across all three problems at once.

## 2. Target Users / Personas

| Persona | Primary pain | Primary value from Sprawl |
|---|---|---|
| **Solo / side-project developer** | Dozens of half-finished repos, no memory of what's safe to delete. | Safe storage reclaim with guaranteed rollback. |
| **ML / data engineer** | Multi-GB `.safetensors`/dataset files dominate disk usage; manual cleanup risks deleting something still in use. | Reproducibility-gated archive instead of guesswork deletion. |
| **OSS maintainer / contractor** | Large number of cloned repos across multiple clients/orgs, inconsistent stacks. | Plugin-extensible stack detection that doesn't require a tool update per ecosystem. |
| **Security-conscious developer** | Exposed or orphaned API keys in old projects, unclear blast radius if revoked. | Local-first secret discovery and blast-radius mapping without forced network exposure. |

## 3. Goals & Success Metrics

| Goal | Metric |
|---|---|
| Reclaim disk space safely | GBs reclaimed via Archive/Nuke actions, with **zero** unrecoverable data-loss incidents reported. |
| Reduce credential exposure risk | Number of previously-undiscovered live secrets surfaced per user in first week of use. |
| Reduce cognitive overhead of project sprawl | Time-to-resurrect: median time from `sprawl analyze --deep` to a working `run.sh` for a previously undocumented project. |
| Maintain trust | Zero incidents of an action executing without the consent gate that was specified for it. |

**North star:** Sprawl should be the kind of tool a security-conscious developer trusts to run unattended, specifically *because* it never takes an irreversible action without explicit, gated consent.

## 4. Non-Goals / Out of Scope (v1.2)

- Not a GUI application — TUI/CLI only for this version.
- Not a multi-machine or cloud-synced tool — all state is local to one machine, except Team Template config, which travels via the user's own VCS.
- Not a replacement for a real backup solution — the manifest/rollback system protects against Sprawl's own actions, not against unrelated data loss.
- Not a production secret-rotation or compliance tool — Token Sentinel discovers and helps assess local secret exposure; it does not manage secret lifecycle in production systems.
- Not a general-purpose AI coding assistant — the local LLM is strictly an escalation mechanism for Sprawl's own internal tasks (stack analysis, ambiguous secret classification), not a chat interface.

## 5. Key User Stories

- *As a developer with 40+ cloned repos, I want to know which ones are safe to delete dependencies from, so I can reclaim disk space without breaking anything I might come back to.*
- *As a developer, I want to be warned about exposed API keys without my key data ever leaving my machine unless I explicitly approve a check.*
- *As an ML engineer, I want bloated dataset/checkpoint directories archived rather than guessed-and-deleted, with a guaranteed way to bring them back.*
- *As a contractor managing client repos in unfamiliar stacks, I want stack detection to work without waiting for an official tool update.*
- *As any user, I want every destructive action to require a deliberate decision on my part — never something that happens because a background process decided it was probably fine.*

## 6. Key Differentiators

| vs. | Sprawl's edge |
|---|---|
| Manual `find`/`du` scripts | Reproducibility-aware, with guaranteed rollback — not just "what's big," but "what's safe to remove." |
| Generic disk-cleaner utilities | Understands project structure (lockfiles, local patches) before suggesting deletion; most disk cleaners don't. |
| SaaS secret-scanning products | Fully local-first; nothing leaves the machine without explicit, per-action consent. |
| Ad hoc AI coding assistants used for cleanup | Heuristics-first by design — AI is a bounded, cold-booted escalation, not the default execution path for routine work. |

## 7. Prioritization & Phasing

**MVP (Phase 1):** Archaeologist Layer 1 (deterministic stack detection, first-party plugins only), Sweeper with the Reproducibility Safety Gate and manifest/restore, Token Sentinel Layer 1 (passive entropy + known-prefix detection, local vault only), core TUI (Dashboard + Sweeper Inbox tabs).

**Phase 2:** Token Sentinel Layer 2/3 (HITL batch inbox, opt-in MCP verification), Archaeologist Layer 2/3 (AI escalation, `analyze --deep`), Semantic Search tab, community WASM plugin support.

**Phase 3:** `resurrect --export-agent`, `profile-machine`, persona preset library expansion, plugin marketplace/discovery conventions.

This phasing exists because the AI escalation paths and plugin ecosystem are explicitly the "5% edge case" tier — they should not block shipping a trustworthy, narrower MVP.

## 8. Key Risks

| Risk | Mitigation |
|---|---|
| CLI/TUI-only interface caps adoption to power users. | Accepted for v1; explicitly out of scope to chase broader adoption before the trust model is proven. |
| A single destructive mistake (an incorrectly-enabled Nuke) would be fatal to user trust. | This is why the Zero-Trust Reproducibility Gate exists as a core, non-negotiable invariant — see the Design Spec and Threat Model. |
| Persona/stack coverage may lag real-world diversity of users' projects. | WASM plugin architecture exists specifically so coverage grows without a Sprawl release cycle. |
