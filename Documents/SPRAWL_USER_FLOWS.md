# Sprawl — User Flow Diagrams

Five flows covering the journeys a real user actually takes through Sprawl, from first install through the highest-risk action in the system. Each diagram's decision points map directly to the consent gates and safety invariants defined in the Design Spec — none of these flows have a path that skips a gate.

---

## 1. First-Run Onboarding

```mermaid
flowchart TD
    A[Install Sprawl binary] --> B[sprawl daemon start]
    B --> C{Run sprawl profile-machine?}
    C -- No --> D[Daemon runs with empty patterns.toml<br/>Global defaults only]
    C -- Yes --> E[WASM detectors scan known project roots]
    E --> F{Unknown stacks found?}
    F -- No --> G[patterns.toml generated<br/>from deterministic results only]
    F -- Yes --> H[Prompt: allow LLM scan of<br/>unrecognized directories?]
    H -- Decline --> G
    H -- Consent --> I[Cold-boot LLM<br/>WAKING INFERENCE CORE...]
    I --> J[Persona inferred,<br/>patterns.toml generated]
    G --> K[sprawl ui: Dashboard tab,<br/>ready for passive monitoring]
    J --> K
```

**Key point:** the LLM step is reachable only through an explicit consent prompt (H); declining is a fully supported path that still produces a usable configuration.

---

## 2. Passive Monitoring → Triage Decision

```mermaid
flowchart TD
    A[Daemon idle, 0% CPU] --> B[Filesystem event or<br/>staleness threshold crossed]
    B --> C[Directory flagged in Sweeper Inbox]
    C --> D[User opens sprawl ui --triage]
    D --> E{User selects action}
    E -- Snooze --> F[Deferred 30 days,<br/>single keypress, immediate]
    E -- Archive --> G[Atomic uncompressed move<br/>+ symlink + manifest entry]
    E -- Nuke --> H{Reproducibility Safety Gate<br/>core re-verification}
    H -- Lockfile present,<br/>no local patches --> I[Modal: explicit second<br/>confirmation required]
    H -- Local patch or<br/>file:// dependency found --> J["[X] Nuke shown locked<br/>with reason inline — no override"]
    I -- Confirmed --> K[Permanent delete executed]
    I -- Cancelled --> C
    G --> L[Reversible via sprawl restore]
```

**Key point:** H is the Zero-Trust invariant — it runs every time, regardless of what any plugin previously reported, and J has no override path.

---

## 3. Token Sentinel: Discovery → Review → Optional Verify

```mermaid
flowchart TD
    A[Passive scan finds high-entropy string] --> B{Matches known<br/>provider prefix?}
    B -- Yes --> C[Hash + AES-encrypt + vault to OS keyring<br/>zero network calls]
    B -- No --> D{Survives negative filter?<br/>UUID/SHA/JWT/noise_patterns}
    D -- Filtered out --> E[Discarded, no inbox entry]
    D -- Survives --> F[Queued in Sentinel Inbox<br/>amber, needs review]
    F --> G{User action}
    G -- Manual tag --> C
    G -- Press W: authorize batch --> H[Cold-boot LLM,<br/>single batch classification pass]
    H --> C
    C --> I{User runs sprawl verify --key ID?}
    I -- No --> J[Key stays vaulted, unverified]
    I -- Yes, opt-in --> K[Delegated to user-installed<br/>read-only MCP server]
    K --> L[Status updated locally]
```

**Key point:** no path from F reaches an LLM without the explicit `[W]` keypress in G — this is the fix for the earlier design flaw where ambiguous strings woke the model automatically.

---

## 4. Resurrecting an Undocumented Project

```mermaid
flowchart TD
    A[Archaeologist flags project as Unknown Stack] --> B[5 unique unknown roots? Drift Alert<br/>passive suggestion to re-profile]
    A --> C[User runs sprawl analyze DIR --deep]
    C --> D[Cold-boot LLM,<br/>reverse-engineers build logic]
    D --> E["Prompt: [L]ocal or [g]lobal<br/>save destination?"]
    E -- Local --> F[.sprawl.toml written<br/>to project root]
    E -- Global --> G[Config cached in<br/>~/.sprawl/cache/, project untouched]
    F --> H{User runs sprawl resurrect DIR --export-agent?}
    G --> H
    H -- Yes --> I[Bundle + diagnostics compiled<br/>to resurrection-kit.md]
    I --> J[Copied to clipboard via arboard<br/>Wayland: detached 60s-TTL helper]
    J --> K[User pastes into external<br/>agentic IDE — Sprawl's role ends here]
```

**Key point:** E is a real fork, not a formality — declining "Local" means Sprawl never writes into a directory the user may have under version control without saying so first.

---

## 5. Installing a Community Plugin

```mermaid
flowchart TD
    A[User finds a community StackDetector plugin] --> B[sprawl plugin install path/URL]
    B --> C[Loaded into wasmtime sandbox<br/>no filesystem access beyond scope, no network]
    C --> D[Plugin now runs at Archaeologist Layer 1<br/>for matching projects]
    D --> E[Plugin returns stack ID +<br/>reproducibility advisory]
    E --> F[Core independently re-verifies<br/>reproducibility — Zero-Trust Gate]
    F --> G{Core agrees with plugin?}
    G -- Yes --> H[Plugin's stack detection used normally]
    G -- No, core finds<br/>a local override plugin missed --> I[Core veto wins regardless —<br/>Nuke stays locked for that project]
```

**Key point:** installing a community plugin only ever grants *advisory* power — F/G/I show that the plugin can never be the sole authority for a destructive action, no matter how it was written or where it came from.
