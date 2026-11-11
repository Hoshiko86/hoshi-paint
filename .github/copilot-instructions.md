# Hoshi Paint Copilot instructions

Review and generate code as a long-lived, cross-platform Rust graphics editor.
Read `docs/ARCHITECTURE.md` before reviewing a major feature or structural
change. Treat its architecture quality gate as required review criteria.

## Review priorities

1. Protect dependency direction: `hoshi-core` is UI and platform independent;
   `hoshi-controller` depends on the core, never on Slint or native adapters;
   presentation and operating-system adapters depend inward.
2. Protect document integrity: edits are atomic history operations, previews are
   isolated from committed pixels, failures preserve the prior valid state, and
   persistence honors its durability contract.
3. Require explicit invariants and domain types. Flag contradictory modes,
   unchecked dimensions or allocation sizes, ambiguous coordinate spaces,
   partial operations, and primitive values that allow invalid domain states.
4. Review modularity using SOLID, separation of concerns, DRY, inversion of
   control, the Law of Demeter, and composition over inheritance. Flag reach-
   through access, platform policy in the domain, duplicated rules, and central
   modules gaining unrelated responsibilities.
5. Check command-query separation, retryable-operation idempotency, least
   authority, and actionable error/progress/cancellation behavior where they
   apply.
6. Preserve shared behavior across Windows, macOS, iOS, and Linux. Platform
   branches belong only in narrow adapters and must expose a common contract.
7. Prefer focused behavioral tests at stable contracts. Flag tests coupled to
   incidental screen coordinates, literal theme colors, private layout, timing,
   or implementation details.

## Review output

- Report only findings that have a concrete correctness, maintainability,
  security, portability, performance, accessibility, or architecture impact.
- Cite the smallest relevant file and line range and describe the triggering
  scenario.
- State the violated invariant or architecture criterion and propose the
  smallest maintainable correction.
- Mark a finding as blocking only when merging would violate a documented
  invariant, dependency rule, data-safety guarantee, or supported-platform
  contract. Label bounded design debt as follow-up instead.
- Do not request speculative abstractions. A new abstraction must correspond to
  an existing responsibility, invariant, or variation point.
- If a criterion is not applicable, accept a clear explanation in the pull
  request. Passing tests alone is not architecture evidence.

Use clear Rust and Slint that a human contributor can understand. Keep unsafe
and platform-specific code small, documented, and isolated behind safe APIs.
