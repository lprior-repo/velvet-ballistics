# vb-1tq2d — independent code review

Reviewer: deepseek-review · Bead: `vb-1tq2d` · status **IN_PROGRESS** · notes **OPEN**.
Scope: fixture-migration code review + source-preservation audit + bounded-fix
confirmation. No production/VCS mutations.

## Verdict: REJECTED (bounded)

The fixture migration `a8452f9e` ("finish distinct run-attempt fixture and symbolic
generator migration") has one in-scope compilation defect; a separate pre-existing
defect blocks the same Kani gate. Both remain present in HEAD.

### Counterexample (in scope, `a8452f9e`)

`crates/vb_core/src/kani_idempotency_gates.rs:169` — `fn symbolic_ticket()` builds
`ActionTicket` with 7 fields, missing `attempt`:

```rust
ActionTicket {
    run: RunId::new(kani::any()),
    step: StepIdx::new(kani::any()),
    seq: SeqNo::new(kani::any()),
    action: ActionId::new(kani::any()),
    run_attempt: kani::any(),
    idempotency_key: kani::any(),
    capacity: kani::any(),
    // MISSING: attempt
}
```

`ActionTicket` (`crates/vb_core/src/action/ticket.rs`) requires 8 fields including
`pub attempt: u16`. `a8452f9e` renamed `attempt` → `run_attempt` **instead of adding**
`run_attempt` alongside `attempt`. The same commit migrated `check_ticket_key_case`
correctly (kept `attempt`, added `run_attempt`), so `symbolic_ticket()` is the sole
miss. Fix: add one line `attempt: kani::any(),`.

Verified exit log (`.beads/vb-1tq2d/evidence/kani-ticket-compile.json`, exit 1):
`error[E0063]: missing field 'attempt' in initializer of 'action::ticket::ActionTicket'`.

### Secondary error (pre-existing, out of 3-commit scope)

`crates/vb_core/src/replay/kani_harnesses.rs:22` — `make_minimal_plan` hardcodes
`WorkflowParts { … }` missing `input_slots` (E0063). Last touched by `31e5182d`
(vb-6jn77). GOD-RULE-1 shape (hardcoded structural input). Bundled into the same
5090 repair lane.

## Verified correct (bounded scope)

- `issue_action_ticket_captures_all_fields` keeps `run_attempt: 2` and per-action
  `attempt: 3` with both `assert_eq!`s.
- `NewActionTicket` field order maps 1:1 to original 8 positional params; no
  duplicate `attempt`/`run_attempt` in `NewActionTicket`, `ActionDispatchContext`.
- Generation vs per-action semantics: `run_attempt = state_run_attempt` at issue,
  `attempt = 1`; retry increments `attempt`, preserves `run_attempt`;
  `compute_idempotency_key(run, run_attempt, seq, action)` mixes generation + seq.
- BCD safe subtraction uses `checked_sub(1)`, retains tamper value, propagates
  rejection; journal unchanged asserted.
- Journal tags: `RetryCheckpoint = 37` valid, `38` rejected, 1–36/40/50 stable.

## Concrete CI counterexample — `assert_ne!` bounded fix (CONFIRMED)

`action_ticket_from_different_run_is_not_equal` (crates/vb_core/src/action/tests.rs)
was rendered vacuous: `78352490`/`c816f6b3`/`6e3c5375`/HEAD build `ticket_a`(run=1)
and `ticket_b`(run=2) with **no `assert_ne!`**. `f4a405469` (vb-k5936) restores
`assert_ne!(ticket_a, ticket_b)`; merge `5766dff91` carries it.

Confirmed bounded and correct: `ActionTicket` derives `PartialEq/Eq/Hash/Debug`; the
tickets differ on `run`, so the assertion is meaningful and the restoration changes
nothing else. This fix is **approved for the assertion only**.

**Not approved:** `5766dff91`'s conflict resolution separately changed two
`let x = runtime.tick_all();` → `runtime.tick_all().ok()` in
`crates/vb_runtime/tests/durable_retry_run_attempt_authority.rs` (lines 264, 314),
discarding the `Result` error in a deterministic-outcome test. Root REJECTED; 5090
is repairing asserted outcomes. Do not adopt `.ok()`.

## Source preservation audit (independent, complete)

All 13 SHAs in `/tmp/vb-wlnl4-source-inventory.json` are `merge-base --is-ancestor`
of `78352490a370`, and each has a meaningful current counterpart (full table in
`coverage.md`). The only naive "missing" files are explained by canonical renames:
`vb_expr/expression.rs` → `expr_*`, and `vb_runtime/tests/*` → `workspace_tests/`.
**No meaningful source dropped.**

## Branch disposition verification

`.beads/vb-1nx31/branch-disposition.{json,md}` is complete and consistent: 44 refs
(integrated 1 · redundant 2 · superseded 7 · blocked 2 · review 28 · contract 3 ·
base 1) + 80 ancestors. Only genuinely-missing source (`bb1245638843`) is
integrated; canonical-digest refs are blocked/owned elsewhere. **No unaccounted
meaningful source.**

## Bead notes: OPEN

Blocking: `symbolic_ticket()` `attempt` (one-line) + `replay` `input_slots`
(5090-assigned). `assert_ne!` restore approved (assertion only); `.ok()` discard
rejected. No production/VCS mutations made.
