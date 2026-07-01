# Contract — vb-vzo9b

## Identifier

- **bead_id**: vb-vzo9b
- **contract_id**: C-vb-vzo9b
- **contract_version**: 1
- **scope**: `fuzz/src/journal_target/readback.rs` lines 183-204 (`fuzz_recovery_decode`)
- **blast_radius**: single test file
- **production_touched**: false (no `crates/**` files, no `Cargo.toml`)
- **fuzz_lane**: true (fuzz harness contract; not Rust-core implementation)

## Parties

| Role | Identifier | Notes |
|---|---|---|
| Issuer | `go-skill` state 1 dispatch | referenced from `STATE.md` |
| Author | `rust-contract` (this agent) | State 3 output |
| Consumer | `proof-planner` (state 4) | owns lane decisions |
| Implementer | `holzman-rust` (state 5+) | owns `readback.rs:196` rewrite |
| Verifier | `formal-verifier` (state 11) | runs `cargo test` / `cargo build` |
| Reviewer | `black-hat-reviewer` (state 12) | signs off |

## Goal

The fuzz body of `fuzz_recovery_decode` must assert the **exact** value of
the `RecoveryRuntimeSummary` returned by `summarize_recovery_events` instead
of the pre-fix disjunctive `summary.run == run || summary.run == RunId::new(0)`.

The exact assertion is the single Rust statement:

```rust
assert_eq!(run_summary, expected_recovery_runtime_summary);
```

This must cover all 11 fields of `RecoveryRuntimeSummary` simultaneously, by
the `PartialEq + Eq + Copy + Debug` derive at
`crates/vb_storage/src/recovery/types.rs:546`.

## Clauses

### C-1 — Exactness of pin

For every legal call to `fuzz_recovery_decode(data: &[u8])` with
`data.len().is_multiple_of() == true`, if
`summarize_recovery_events(&events)` returns `Ok(hydration)`, then:

```
hydration.summary() == RecoveryRuntimeSummary {
    run:             run,         // constructed from data[0]
    first_seq:       seq,         // EventSeq::new(1)
    last_seq:        seq,         // EventSeq::new(1)
    workflow:        Some(digest),
    steps_started:   0,
    steps_succeeded: 0,
    actions_scheduled: 0,
    actions_resolved:  0,
    suspensions:     0,
    slots_written:   0,
    terminal:        None,
}
```

If any of the 11 fields disagree, `assert_eq!` panics with a `Debug`-formatted
diff.

### C-2 — Sentinel rejection

The pre-fix disjunctive acceptance of `RunId::new(0)` is **forbidden** in the
non-empty branch. Post-fix body **must not** introduce any `||`-shaped
acceptance in the non-empty branch. Specifically, the disjunction

```
assert!(summary.run == run || summary.run == RunId::new(0))
```

is replaced by the **strictly stronger** `assert_eq!` from C-1.

### C-3 — Empty-events path unchanged

For `data.len() % 2 != 0` (odd-length data ⇒ empty `events`), the fuzz body
**must** continue to rely on `assert_typed_recovery_error` to sink the
`RecoveryError::NoRecoveryData { run: RunId::new(0) }` returned by
`summarize_recovery_events`. No `assert_eq!` is permitted in this branch
because no `Ok(hydration)` value exists here.

### C-4 — Frame-seed call unchanged

The second production call,

```rust
if let Err(error) = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events) {
    assert_typed_recovery_error(error);
}
```

**must not** be modified. The error sink is the contract for that call; the
`Ok` path is intentionally not asserted in this fuzz body (out-of-scope for
vb-vzo9b).

### C-5 — No production-code change

`crates/vb_storage/src/recovery/replay/summary/apply.rs`,
`crates/vb_storage/src/recovery/replay/summary/derive.rs`,
`crates/vb_storage/src/recovery/replay/summary/accumulator.rs`,
`crates/vb_storage/src/recovery/types.rs`,
`crates/vb_storage/src/recovery/replay/summary/tests.rs`,
`fuzz/Cargo.toml`, `fuzz/src/bin/recovery_decode.rs`, `fuzz/src/lib.rs`,
`fuzz/src/journal_target.rs`, `fuzz/src/journal_target/errors.rs`,
`fuzz/src/journal_target/event.rs` are **read-only** for this bead.

### C-6 — No new error variant, no new type

The post-fix body introduces no new error variant, no new type, no new
`unsafe`, no `unwrap`/`expect`/`panic` outside the desired `assert_eq!` panic.

### C-7 — Closure commands (downstream gate)

The implementer must, after applying the fix:

| Command | Expectation |
|---|---|
| `cargo build -p fuzz --bin recovery_decode` | exit 0 |
| `cargo test -p vb_storage --lib summarize_recovery_events` | all green |
| `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` | all green |

`moon ci` is canonical but not strictly required for this bead (deferred to
landing per bead workflow, `delivery-scope.jsonl`).

### C-8 — Forbidden patterns

The post-fix body **must not** contain any of the following in the non-empty
branch:

| Pattern | Reason |
|---|---|
| `assert!(... || ...)` over `RecoveryRuntimeSummary` fields. | Reintroduces the disjunctive defect. |
| `matches!(summary, RecoveryRuntimeSummary { run, .. })`. | Only checks `run`. |
| Field-by-field `assert!` chain (e.g. 11 separate `assert!(...)`). | Brittle, easy to drop a field. |
| `let _summary = ...;` (no assertion). | Coverage-only fuzz target. |
| `dbg!(...)` instead of `assert_eq!`. | Failure mode is silent. |
| `unwrap()` / `expect()` on `RecoveryResult`. | Disallowed by Holzman Rust. |

## Strong-Pattern Reference

`crates/vb_storage/src/recovery/replay/summary/tests.rs:285-302` uses
`matches!(result, Err(RecoveryError::NoRecoveryData { run }) if run == RunId::new(0))`
to pin a specific field against a contract. The post-fix fuzz body uses the
same principle but leverages `RecoveryRuntimeSummary`'s derive set for a
structurally stronger `assert_eq!(value, expected)`.

## Mapping to Proof Seeds

See `proof-seeds.jsonl`. C-1 is bound to seed `PS-vb-vzo9b-1`; C-2 binds to
`PS-vb-vzo9b-2`; C-3 binds to `PS-vb-vzo9b-3`; C-5/C-6 bind to
`PS-vb-vzo9b-4`; C-8 binds to `PS-vb-vzo9b-5`.

## Mapping to Traceability

See `traceability-matrix.jsonl`. Each contract clause maps to one or more
field-level assertions (`run`, `first_seq`, ..., `terminal`) with proof-seed
references and risk tags.

## Open Domain Questions

Documented in `domain-model.md` and `codebase-map.md §8`:

1. Should the fuzz payload be extended to cover multi-run divergence and
   `EventSeq::MAX` overflow? (Out-of-scope for vb-vzo9b.)
2. Should a deterministic `#[test]` wrapper be added? (Out-of-scope.)
