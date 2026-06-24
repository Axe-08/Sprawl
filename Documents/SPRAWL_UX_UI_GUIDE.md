# Sprawl — UI/UX Design Guide (Terminal Interface)

## 1. Design Principles

Sprawl's interface is a TUI, not a GUI — every convention below is chosen for a terminal context specifically, not adapted from web/desktop patterns.

1. **Calm by default, loud when it matters.** The daemon is silent and invisible during normal operation. The moment something needs human judgment (a triage decision, an ambiguous secret, an insufficient-resource error), the UI should be unambiguous and impossible to miss.
2. **Friction should be proportional to consequence.** A `Snooze` costs one keypress. A permanent `Nuke` should never feel as cheap as a `Snooze`, even when it's technically permitted.
3. **Color is a safety signal, not decoration.** Every color choice below maps to a specific risk meaning and is used consistently across all four tabs.
4. **No silent state changes.** If a configuration layer overrides another, if a plugin's verdict was vetoed, or if an action was deferred — the UI says so, every time, not just on first occurrence.

## 2. Color Semantics

| Color | Meaning | Example use |
|---|---|---|
| Red | Irreversible action, currently blocked or requires explicit override friction | Disabled `[X] Nuke` state; checksum failure errors |
| Amber / Yellow | Needs human review, not yet classified | Sentinel Inbox ambiguous-string entries |
| Green | Verified safe / action completed successfully | Reproducibility Gate passed; restore completed |
| Cyan / Blue | Informational, no action required | Dashboard metrics, drift alerts |
| Gray | Dormant, snoozed, or explicitly disabled | Snoozed triage items; disabled plugin |

**Accessibility note:** color is never the *only* signal. Every color-coded state also has a distinct symbol or label (see §3) so the interface remains legible under color-blind-safe palettes or in terminals with limited color support.

## 3. Iconography & Symbol Conventions

| Symbol | Meaning | Used in |
|---|---|---|
| `[X]` | Permanent delete action | Sweeper Inbox |
| `[A]` | Archive (reversible) action | Sweeper Inbox |
| `[S]` | Snooze | Sweeper Inbox |
| `[W]` | Authorize a cold-boot AI task | Sentinel Inbox, Archaeologist deep-analyze prompt |
| `[L]` / `[g]` | Local vs. global config write destination | Archaeologist save prompt |
| `⚠️` | A configuration layer was silently overridden by a more specific one — now made visible | Sweeper Inbox (Visual Override Trace) |
| `🔒` (or text equivalent in low-color terminals: `[LOCKED]`) | Action permanently disabled by the Reproducibility Safety Gate | Sweeper Inbox |

Symbols are kept consistent across every tab — `[X]` always means "permanent and destructive" everywhere it appears, never reused for a different meaning elsewhere in the interface.

## 4. Layout & Navigation

- A persistent tab bar (Dashboard / Sweeper Inbox / Sentinel Inbox / Semantic Search) is always visible; the active tab is the only thing that changes.
- A footer hotkey reference is always visible and reflects *only* the actions valid in the current context — a disabled `[X]` is shown grayed-out with the reason inline (e.g. `[X] Nuke (locked: local patch detected)`), never simply omitted, so the user learns *why* without having to ask.
- Modal confirmation is reserved for the highest-risk actions only (permanent Nuke when the Gate allows it); lower-risk actions (Archive, Snooze) act immediately on keypress to keep the interface responsive and avoid confirmation fatigue that would make users blindly click through real ones.

## 5. Confirmation Friction Tiers

| Tier | Action | Friction |
|---|---|---|
| Low | Snooze | Single keypress, immediate, undoable by re-surfacing later |
| Medium | Archive | Single keypress, immediate — but logged to the manifest and reversible via `restore` |
| High | Nuke (when Gate allows) | Single keypress triggers a modal requiring a second, explicit confirmation keystroke before executing |
| Absolute | Resurrect script execution without a sandbox | No override exists. The action is refused outright, not just made harder. |

## 6. Progress & Waiting States

The interface must never appear frozen. Established pattern, used consistently for any operation with real latency:

```
[WAKING INFERENCE CORE...] ▓▓▓▓▓▓░░░░ 62%  (loading model weights from disk)
```

This pattern — verb-first status text, a progress indicator, and a one-line explanation of what's actually happening — is the house style for every multi-second operation (model cold-boot, large file archive move, deep repository scan), not just the AI escalation path.

## 7. Empty States

Each tab has a defined empty state so first-run users aren't staring at a blank screen wondering if something is broken:

- **Dashboard, no projects indexed yet:** a one-line prompt suggesting `sprawl profile-machine`.
- **Sweeper Inbox, nothing flagged:** "Nothing to triage right now — Sprawl will surface candidates here as projects go dormant."
- **Sentinel Inbox, no ambiguous strings queued:** "No ambiguous secrets pending review."
- **Semantic Search, no index built yet:** explains that background indexing runs only during idle time and gives an estimate or a manual trigger hint.

## 8. Error & Warning Treatment

House style, established by the RAM pre-flight error in the Design Spec and applied everywhere: **state what's wrong, give the specific numbers, state what to do next.** Never a bare failure code.

> `[!] INSUFFICIENT HEADROOM: 3.1GB required, 2.8GB available. Aborting cold-boot.`

Not:

> `Error: operation failed (code 12)`

## 9. Theming

The TUI inherits the active terminal's color scheme rather than imposing its own palette, so it tiles naturally in window-managed environments. Color *roles* (§2) are mapped onto whatever the terminal's ANSI palette provides — Sprawl defines semantic roles, not literal hex values, so it adapts cleanly across light/dark and custom terminal themes.

## 10. Cross-reference

See `SPRAWL_USER_FLOWS.md` for how these UI states connect across a full task from trigger to resolution.
