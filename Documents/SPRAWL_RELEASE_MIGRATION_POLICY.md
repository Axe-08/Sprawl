# Sprawl — Release & Migration Policy

## 1. Versioning Scheme

The Sprawl binary follows Semantic Versioning (`MAJOR.MINOR.PATCH`).

| Change type | Version bump |
|---|---|
| New default model, or updated checksum/mirror for the same model | Minor, at minimum |
| Breaking change to the WIT plugin ABI (major version bump per Plugin ABI Spec §5) | Major |
| Breaking change to `patterns.toml` / `.sprawl.toml` schema (requires migration) | Major |
| New non-breaking config field, new CLI subcommand, new persona preset | Minor |
| Bug fixes, performance improvements, no interface/schema change | Patch |

## 2. What Triggers a Release

- **Model updates require a new release**, full stop — the download URL and SHA-256 checksum are hardcoded into the binary (Design Spec §3), so there is no independent model-update channel. This is a deliberate tradeoff for download integrity (ADR-012 context) over update flexibility.
- **WIT ABI major-version changes** require a release that updates the host's supported-version range; plugins built against the old major version are refused with a clear error, not silently miscompiled against.
- **Config schema changes** require a release that ships the corresponding migration logic (§3 below) before any user can encounter the new schema version.

## 3. Migration Process

1. Before applying any schema migration, the daemon **backs up `ledger.db`, `manifest.json`, and all `patterns.toml`/`.sprawl.toml` files it's about to touch** to a timestamped backup directory under `~/.sprawl/backups/`.
2. Migrations run in a dry-run mode first when feasible (compute the migrated result, diff against current, log the diff) before committing.
3. If a migration fails partway, the daemon restores from the pre-migration backup automatically and refuses to start in the new schema state — it does not attempt partial operation against a half-migrated config.
4. Migration outcomes (success, failure + rollback, dry-run diff) are logged locally with timestamps for post-hoc inspection — see the Operational Runbook for what a user does if a migration fails.

## 4. Deprecation Policy

- **Config schema versions:** an old `schema_version` is auto-migrated on load for at least two major Sprawl versions before the daemon refuses to load it outright and requires a manual export/migration path.
- **WIT plugin ABI versions:** a deprecated major version is refused immediately on the release that bumps the major version — there is no grace period for plugin ABI, since silently running against a stale interface is a correctness risk, not just an inconvenience.
- **CLI subcommands:** deprecated commands print a warning pointing to the replacement for at least one minor version cycle before removal.

## 5. Release-Gating CI Requirements

A release build cannot ship unless:
- The full cross-platform CI matrix (Linux/macOS/Windows) passes, per the Test Plan.
- The Reproducibility Safety Gate test matrix (Test Plan §3) passes in full — this is never waived.
- `cargo-audit` reports no unresolved high/critical advisories in the dependency tree.
- Binary size is within the 45MB budget (excluding model weights).
