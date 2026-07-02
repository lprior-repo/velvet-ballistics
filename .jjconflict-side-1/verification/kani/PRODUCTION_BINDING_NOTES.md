# Kani Production-Binding Notes (vb-8sa4i, vb-frskm, vb-xy2aw)

**STATUS:** No Kani harness in `verification/kani/` is currently bound to
production via `#[path = ".../crates/..."]`. Until a binding is added,
no Kani harness may be cited as proof of a production obligation.

## Required Production-Binding Pattern

A production-bound Kani harness must use one of:

1. `#[path = ".../crates/<crate>/src/<path>.rs"]` at the top of the harness
   file or in a module declaration.
2. A `production_inner/<path>.rs` mirror with a drift-gate header citing
   the production revision SHA, included via `#[path]`.
3. A companion `extern_<name>.rs` that itself binds to production.

A harness that defines its own copy of `WorkflowFrame`,
`RunState`, `ActionTicket`, or any other production struct is by
definition NOT bound and is downgraded.

## Current Inventory (audit-trail)

| Artifact | Path | Binding | Reason |
|---|---|---|---|
| `kani_list/` | `.evidence/kani-list/*.json` | (raw `cargo kani --list` snapshots) | Not a harness; raw evidence only. |
| `verification/kani/` | `verification/kani/**` | none yet | Refinement shadow models, audit-only. |

## Forbidden Patterns

### `cover!`-only harnesses (vb-frskm)

```rust
#[kani::proof]
fn check_no_panic() {
    // ... exercise some code ...
    kani::cover!(true); // does NOT prove any property
}
```

These harnesses are downgraded. They MAY be retained for "did this code
path execute?" coverage but MUST NOT be cited as proof of an obligation.

### `assert!(true)` harnesses

```rust
#[kani::proof]
fn check_something() {
    let x = kani::any::<u32>();
    assert!(true); // vacuous
}
```

Downgraded. Replace with a real assertion against production behavior.

### Hardcoded shapes (vb-xy2aw)

```rust
#[kani::proof]
fn check_workflow_frame_safe() {
    let frame = WorkflowFrame {
        slot_count: 16,             // hardcoded!
        symbols_count: 4,           // hardcoded!
        // ...
    };
    assert!(frame.is_safe());
}
```

Downgraded. Replace with `kani::any()` generators that span the
production input domain.

## Resolution Applied

1. Every Kani harness under `verification/kani/` is documented as
   SCOPED-ONLY until bound. See `verification/FLUX_KANI_VERUS_HYGIENE_NOTES.md`
   for the bead-level decisions.
2. Future harnesses MUST add a `#[path = ".../crates/..."]` binding or
   be flagged as RETIRE in their file header.

## Follow-Up Beads

- **KANI-BIND-001**: bind the existing `verification/kani/` artifacts
  to production. Each artifact gets its own follow-up bead ID.
- **KANI-COVER-001**: replace `cover!`-only harnesses with real
  assertions.
- **KANI-SHAPE-001**: replace hardcoded shapes with `kani::any()`
  generators.