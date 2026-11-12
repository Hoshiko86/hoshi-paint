# Architecture Decision Records

ADRs record decisions that shape more than one task or crate. They sit under
the binding quality gate in [`../ARCHITECTURE.md`](../ARCHITECTURE.md) and the
implementer rules in [`../../AGENTS.md`](../../AGENTS.md). Where an ADR and
those documents disagree, the quality gate wins and the ADR must be amended.

| # | Title | Status | Unblocks (roadmap) |
| --- | --- | --- | --- |
| [0001](0001-selection-model.md) | Selection model | Accepted | 0.1.0 selections, pixel movement, crop |
| [0002](0002-effect-framework.md) | Effect and adjustment framework | Accepted | 0.3.0 adjustments, 0.4.0 effects, 0.7.0 Wasm |
| [0003](0003-tool-framework.md) | Tool framework and stable tool IDs | Accepted | 0.0.0 brush/eraser, 0.2.0 tools |
| [0004](0004-pixel-precision-blend-modes.md) | Pixel precision and blend modes | Accepted | 0.5.0 blend modes, layer properties |
| [0005](0005-platform-services.md) | Platform services | Accepted | 0.1.0 clipboard, 0.2.0 pressure, 0.6.0 text |

Suggested order of work: 0003 (unblocks DRAW-1 cleanly) → 0001 → 0004 → 0002 →
0005. 0001 and 0004 both touch `Layer`/`State`/history serialization; land
their core slices one at a time.

## Rules

- **Numbering:** four digits, never reused. File name `NNNN-short-slug.md`.
- **Status:** `Proposed` → `Accepted` (owner approves) → optionally
  `Superseded by NNNN` or `Deprecated`. Accepted ADRs are edited only to fix
  typos or add a "Superseded" link; a changed decision is a new ADR.
- **Scope:** one decision family per ADR, ≤ ~150 lines. Type sketches are
  illustrative; names may change in implementation, contracts may not.
- **Evidence:** claims about Paint.NET behaviour or third-party crates cite a
  source and the date checked. Unverified claims are marked *(unverified)* and
  listed under Open questions.
- **Slices:** every ADR ends with Codex-sized tasks (1–2 files, one concern,
  explicit dependencies) that can be copied into `codex-task` issues.

## Shared constraints (apply to every ADR)

- Dependency direction: `hoshi-core` ← `hoshi-controller` ← desktop adapter.
  Core has no UI, OS, clock or thread dependency.
- No `Arc<Mutex>`/`RwLock` for shared state. UI is single-threaded with `&mut`.
  Background work uses `std::sync::mpsc` or `std::thread::scope` over disjoint
  tiles; workers read immutable `Arc` snapshots. A cancel flag may be an
  `Arc<AtomicBool>` (precedent: `screen_pick.rs`).
- Typed `thiserror` errors; no `unwrap`/`expect`/`panic!` in non-test code.
- Atomic edits: failure leaves the prior state; one user action = one history
  entry; previews never write committed pixels.
- Every pixel/scratch allocation reserves the global memory budget.
- CPU reference results are deterministic and independent of thread count.

## Template

```markdown
# NNNN. Title

- Status: Proposed
- Date: YYYY-MM-DD
- Deciders: owner (approve), Claude (architect)
- Related: issues, ADRs, docs

## Context
What forces the decision: current code, Paint.NET behaviour (cited), limits.

## Decision
The chosen design, stated decisively. Type sketches allowed.

## Alternatives considered
Each option with the reason it lost.

## Consequences
Positive, negative, and follow-up work.

## Implementation slices
1. **Slug** — files (1–2) · depends on · done when.

## Parity notes
Paint.NET features this unblocks, and known deliberate differences.

## Open questions
Decisions the owner must make; unverified facts to check.
```
