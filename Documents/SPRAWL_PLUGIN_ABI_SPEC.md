# Sprawl — Plugin ABI Specification (WIT) & Versioning Policy

## 1. Why a Formal ABI

"Compiles to WASM" is not an interface contract. Rust, Go, and C plugin authors need a single, typed, language-agnostic definition of what a `StackDetector` plugin receives and must return. Sprawl uses the **WASM Component Model**, defined via **WIT (WASM Interface Types)**, for this contract.

## 2. The `StackDetector` Interface (Illustrative)

```wit
package sprawl:stack-detector@1.0.0;

interface detector {
    record dependency {
        name: string,
        version-spec: string,
        is-local-path: bool,   // true if file:// / link: / path-based
    }

    record reproducibility-verdict {
        is-reproducible: bool,
        evidence: list<string>,   // e.g. ["yarn.lock found", "no patch-package dir"]
    }

    record stack-info {
        ecosystem: string,        // e.g. "node", "rust", "python", "go"
        entry-points: list<string>,
        dependencies: list<dependency>,
        reproducibility: reproducibility-verdict,
    }

    // Host grants read-only, scoped filesystem access to the directory under inspection.
    // No network capability is ever granted to a plugin.
    detect: func(project-root: string) -> option<stack-info>;
}

world stack-detector-plugin {
    export detector;
}
```

This is illustrative, not final syntax — the binding authority is whatever `.wit` file ships in the Sprawl repository at release time.

## 3. Capability Model

| Capability | Granted to plugin? |
|---|---|
| Read files within the declared project root | Yes (sandboxed, read-only) |
| Write to any filesystem location | No |
| Network access | No, ever |
| Read files outside the declared project root | No |
| Spawn processes | No |

A plugin's `reproducibility-verdict` is **advisory input** to the Sweeper, not an authority. Per ADR-009, the Rust core independently re-verifies the same filesystem evidence itself before any Nuke action is enabled — a plugin cannot grant a destructive action on its own verdict alone.

## 4. Signing & Trust Tiers

| Tier | Distribution | Trust level |
|---|---|---|
| First-party | Shipped with the Sprawl binary (Node, Rust, Python, Go) | Cryptographically signed against a key embedded in the binary; verified at load |
| Community | `sprawl plugin install <path/URL>` | Unsigned by default; sandboxed and treated as advisory only — never the sole authority for a destructive action regardless of trust tier |

*Open item carried from prior review:* community plugin downloads via URL do not yet have a checksum/signature verification step at install time. Recommended before GA — see Known Limitations in the Design Spec.

## 5. Versioning Policy

- The WIT package is versioned with SemVer (`sprawl:stack-detector@MAJOR.MINOR.PATCH`).
- **Major version bump:** any breaking change to the interface shape (removed/renamed fields, changed function signatures). Sprawl refuses to load a plugin declaring a major version it doesn't support, with a clear error naming the expected version.
- **Minor version bump:** additive, backward-compatible changes (new optional fields). Sprawl loads the plugin and warns if the plugin declares an older minor version than the host expects, since the plugin simply won't populate newer optional fields.
- **Patch version:** no interface shape change; documentation/clarification only.
- Sprawl's host-side ABI version is checked against the plugin's declared `world` version at load time, before any plugin code executes.

## 6. Plugin Lifecycle in the Host

1. `sprawl plugin install` copies the `.wasm` component into `~/.sprawl/plugins/`.
2. On daemon start (or `plugin list` refresh), each plugin's declared interface version is checked against the host's supported range.
3. Plugins are instantiated fresh per detection call inside a `wasmtime` sandbox with resource limits (fuel/memory) applied — not left resident indefinitely.
4. A plugin's resource limit violation or panic is caught at the host boundary and surfaced as a failed detection for that project, not a daemon crash.
