# Sprawl — Threat Model

## 1. Scope & Approach

This document enumerates trust boundaries and threats specific to Sprawl's architecture, using a lightweight STRIDE-style pass per boundary rather than a full formal model. It assumes the user's machine and OS are not already compromised — local privilege escalation and OS-level attacks are out of scope (see §5).

## 2. Assets to Protect

| Asset | Where it lives | Sensitivity |
|---|---|---|
| Discovered API keys / secrets | OS keyring (vaulted), hashed reference in `ledger.db` | High — direct credential exposure if leaked |
| User source code | Read in-place; temporarily passed to local LLM during `analyze --deep`/`bundle`; explicitly exported via `resurrect --export-agent` | Medium-high — IP/confidentiality concern, not a credential |
| User filesystem (sweep targets) | Local disk | High — destructive actions act directly on it |
| `ledger.db` / `manifest.json` | `~/.sprawl/` | Medium — integrity matters for trustworthy rollback |

## 3. Trust Boundaries & Threats

### Boundary: User ↔ TUI/CLI
- **Threat (Spoofing N/A — local only):** Not applicable; this is a local single-user terminal session.
- **Threat (Tampering):** A confused or rushed user could authorize a destructive action without understanding the consequence.
- **Mitigation:** Confirmation friction proportional to risk (see UX Guide §5); the Gate in the next boundary is the real backstop, not user attentiveness alone.

### Boundary: Core Daemon ↔ WASM Plugin
- **Threat (Tampering):** A plugin — buggy or actively malicious — returns a false `is_reproducible = true` verdict to get a destructive action approved.
- **Mitigation:** The Zero-Trust Reproducibility Gate (ADR-009) — the core independently re-scans for lockfile/patch/local-path evidence and can veto the plugin. This is the single load-bearing mitigation in the entire system; no other control substitutes for it.
- **Threat (Denial of Service):** A plugin consumes excessive CPU/memory while running.
- **Mitigation:** `wasmtime` resource limiting (fuel metering / memory limits) should be configured per plugin invocation — flagged as a build requirement, not optional hardening.
- **Threat (Elevation of Privilege):** A plugin attempts to escape the WASM sandbox to gain filesystem or network access beyond its declared capabilities.
- **Mitigation:** Relies on `wasmtime`'s sandboxing guarantees; the runtime must be kept current with upstream security patches as a release-process requirement (see Release & Migration Policy).

### Boundary: Core Daemon ↔ Model Download Source
- **Threat (Tampering):** A corrupted or maliciously substituted model blob is downloaded and loaded for inference.
- **Mitigation:** Hardcoded SHA-256 checksum verification before load; cryptographically verified fallback mirror; checksum failure purges the file and aborts rather than degrading gracefully into running on unverified weights.
- **Threat (Spoofing):** A DNS-level or MITM attack substitutes the download source.
- **Mitigation:** Checksum verification (above) catches a substituted payload regardless of how the substitution occurred; this is the actual control, not transport trust.

### Boundary: Core Daemon ↔ MCP Server (opt-in network verification)
- **Threat (Spoofing):** A malicious or compromised MCP server impersonates a legitimate provider-verification service.
- **Mitigation:** The user explicitly selects and installs the MCP server themselves (no auto-discovery); Sprawl treats this as a user-trust decision it does not arbitrate, consistent with the "user installs and audits independently" model.
- **Threat (Information Disclosure):** The key material sent for verification is exposed to whatever server the user chose.
- **Mitigation:** This is the explicit, accepted cost of opt-in verification — only triggered per-key, per-user-action, never automatically.

### Boundary: User ↔ External Agentic IDE (via `resurrect --export-agent` / clipboard)
- **Threat (Information Disclosure):** Source code, framework diagnostics, and potentially `.env` references are bundled and placed on the clipboard for pasting into a third-party tool.
- **Mitigation:** This is a deliberate, user-initiated data flow — Sprawl's responsibility ends at clearly informing the user what's being exported and to where it's headed (the clipboard), not at controlling what the destination application does with it.

### Boundary: Core Daemon ↔ Local Filesystem (Sweeper)
- **Threat (Repudiation):** A user disputes that they authorized a given Archive/Nuke action.
- **Mitigation:** The append-only manifest (`manifest.json`) provides a timestamped audit trail of every action taken, sufficient to reconstruct what happened and when.

## 4. Threats Considered and Explicitly Accepted

- **Supply chain risk in Cargo dependencies themselves** (a compromised crate in the dependency tree). Mitigation: `cargo-audit` in CI as a release-gating check (see Release & Migration Policy) — not eliminable, only monitored.
- **A user running an unsandboxed `resurrect` script anyway by manually copying it out and running it themselves.** Sprawl's refusal (ADR-003) prevents Sprawl from doing this *for* the user; it cannot prevent a user from bypassing the tool entirely on their own machine.

## 5. Explicitly Out of Scope

- Attacks where the adversary already has arbitrary code execution as the local user (Sprawl cannot meaningfully defend against an attacker who already controls the account it's running under).
- Physical access / full-disk encryption — assumed to be the OS's and user's responsibility, not Sprawl's.
- Multi-user machine isolation — Sprawl's data model assumes a single-user workstation context.
