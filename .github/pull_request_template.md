## Behavior

Describe the concrete change and link its issue.

## Validation

List checks actually run and platforms tested. Describe untested behavior.

## Architecture fitness

Required for every major feature or structural change. For each group, provide
concrete evidence or explain why it is not applicable. Record exceptions and a
follow-up issue rather than silently accepting debt.

- [ ] **Boundaries:** SOLID, separation of concerns, dependency direction, and
      inversion of control.
- [ ] **Coupling:** Law of Demeter, composition over inheritance, and least
      authority.
- [ ] **State:** atomicity and applicable ACID guarantees, explicit invariants,
      and invalid states made unrepresentable.
- [ ] **Behavior:** DRY, command-query separation, and idempotency for retryable
      operations.
- [ ] **Operations:** observability, actionable errors, progress, and
      cancellation where applicable.

Evidence, exceptions, and follow-up issues:

## Limitations

Record known limits and follow-up work, including format compatibility if affected.
