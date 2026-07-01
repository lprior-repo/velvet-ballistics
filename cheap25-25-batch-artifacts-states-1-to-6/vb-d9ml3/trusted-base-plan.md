# Trusted Base Plan — vb-d9ml3 (Storage trim/snapshot key length cap, P1)

> Schema companion to `proof-obligations.planned.jsonl`. This ledger
> documents every trust marker (assumptions, model reductions, trusted
> abstractions) raised by the four planned obligations. The
> `trusted_base_refs` arrays on each obligation row reference the IDs
> in this plan.

Bead ID: `vb-d9ml3`
Planner invocation: `proof-planner-vb-d9ml3-state4`
Workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3`
Owner state: 4
Captured: 2026-07-01

---

## Trust marker inventory

This bead is a low-blast-radius internal fix (numeric/cap refinement against
an already-bounded `JOURNAL_KEY_BYTES = 17`). The trust surface is small;
one trust marker is required (the const-alias chain), and no Miri
specialist scoping note is needed (no `unsafe` risk tags).

| Note ID | Type | Severity | Owner | Compensating obligation |
|---|---|---|---|---|
| `TB-CAP-001` | Const-alias chain | compile-time | holzman-rust | PO-001-UNIT |

There are no other trust markers because:
- No `unsafe` blocks are added (the entire `vb_storage` crate is
  `#![forbid(unsafe_code)]` per `crates/vb_storage/src/lib.rs`).
- No `assume`/`axiom`/`admit`/`external_body`/`sorry`/`#[trusted]`/`#[ignore]`/`opaque`/`extern_spec` markers are introduced.
- No model reductions (TLC symmetry, Miri `-Z` flags, loom preemptions) are in scope.
- No `cfg_attr(miri, ignore)` tests are added.
- No `kani::cover!` is used as property evidence (PO-003-PROPTEST uses
  the proptest macro framework, not Kani).

---

## TB-CAP-001 — Const-alias chain (MAX_TRIM_KEY_LEN, MAX_SNAPSHOT_KEY_LEN)

### Surface

The new `pub(crate) const` aliases at
`crates/vb_storage/src/constants.rs:74-79` declare:

```rust
pub(crate) const JOURNAL_KEY_BYTES: usize = 17;
pub(crate) const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;
pub(crate) const MAX_SNAPSHOT_KEY_LEN: usize = JOURNAL_KEY_BYTES;
```

### Trust claim

The `pub(crate) const` alias chain enforces compile-time equality
between `MAX_TRIM_KEY_LEN`, `MAX_SNAPSHOT_KEY_LEN`, and
`JOURNAL_KEY_BYTES`. Any future change to `JOURNAL_KEY_BYTES`
propagates to both aliases and to the three magic-17 call sites
(`trimming/logic.rs:36, 77, 222`). The alias chain is the canonical
RHS reference, not a literal `17`, so the const-equality property is
discharged by the Rust type checker at compile time.

### Why this is a trust marker (and not a verification)

The alias chain is **discharged by the compiler** (via `cargo check
-p vb_storage --lib`); the unit test on `PO-001-UNIT` is a redundant
regression check, not the primary mechanism. The trust marker is the
RHS reference itself: the planner commits to a `const X = Y` form, not
a `const X = 17` literal form. If a future contributor inlines the
literal `17` at the alias site, the chain breaks silently — only the
unit test would catch the regression.

### Bound

The bound is one (1) `pub(crate) const` declaration per alias, at the
existing `JOURNAL_KEY_BYTES` declaration site
(`crates/vb_storage/src/constants.rs:74`). The aliases are co-located
with `JOURNAL_KEY_BYTES` to keep the journal key envelope byte
length and the named caps visually adjacent.

### Verification

The unit test on `PO-001-UNIT` exercises the chain:

```
PROPTEST_CASES=10 cargo test -p vb_storage --lib max_key_len_aliases_equal_journal_key_bytes
```

The test asserts:

```rust
proptest!(proptest_config(ProptestConfig::with_cases(10)), |(_unit in 0u8..1u8)| {
    prop_assert_eq!(MAX_TRIM_KEY_LEN, JOURNAL_KEY_BYTES);
    prop_assert_eq!(MAX_SNAPSHOT_KEY_LEN, JOURNAL_KEY_BYTES);
});
```

The strategy is degenerate (`0u8..1u8`); the `PROPTEST_CASES=10`
budget is recorded per the schema's resource-governance gate but
exercised only for completeness — the const equality is a single-shot
property that does not benefit from randomization.

### Compensating evidence

- `PO-001-UNIT` (cargo test invocation; regression gate)
- `cargo check -p vb_storage --lib` (compile-time discharge of the
  const chain; canonical Rust type checker)
- `PO-004-LINT` (`moon run :lint-src` + `rg` static check; ensures
  the magic-17 literal is fully replaced at the three call sites)

### Failure mode

If the alias chain breaks (e.g., a future contributor inlines
`const MAX_TRIM_KEY_LEN: usize = 17;` instead of
`const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;`), the unit
test on `PO-001-UNIT` fails and the bead's contract is violated.
The fix is to restore the `const X = JOURNAL_KEY_BYTES` form.

### Cross-reference

- `proof-strategy.md` §5.1 (REQ-CAP-001 strategy)
- `proof-coverage-matrix.md` (REQ-CAP-001 row)
- `proof-obligations.planned.jsonl::PO-001-UNIT.trusted_base_refs = ["TB-CAP-001"]`
- `contract.md` CC-CAP-001 (named-cap aliases)

---

## Miri specialist scoping note

**Not applicable.** The bead does not introduce any `unsafe` blocks,
FFI, raw pointers, aliasing/provenance, or `repr(C)` layout
assumptions. The `vb_storage` crate is `#![forbid(unsafe_code)]`
and the `unsafe-audit` task at `.moon/tasks/all.yml:87-109` would
catch any new `unsafe` block at the source-lint stage. No Miri
scoping note is required.

---

## Assumptions ledger

| Obligation | Assumption | Resolved by |
|---|---|---|
| PO-001-UNIT | `JOURNAL_KEY_BYTES == 17` (the source-of-truth constant) | Const declaration at `constants.rs:74`; compiler-enforced. |
| PO-001-UNIT | `MAX_TRIM_KEY_LEN` and `MAX_SNAPSHOT_KEY_LEN` are declared as `pub(crate) const` aliases (not `pub const`) | Const declaration at `constants.rs:74-79`; compiler-enforced. |
| PO-001-REGRESSION | `TrimError::INCOMPLETE_TRIM_CODE == 0x4102` (the diagnostic code) | Const declaration at `trimming/mod.rs:62`; compiler-enforced. |
| PO-001-REGRESSION | `JournalError::Trim(inner).diagnostic_code()` delegates to `inner.diagnostic_code()` | Method impl at `error/codes.rs:167`; compiler-enforced. |
| PO-002-INTEGRATION | `temp_journal()` is a working test helper | Helper at `snapshot_tests.rs:16-20` and `trimming/tests.rs:17`; tested. |
| PO-002-INTEGRATION | `journal.run_snapshot.insert(...)` and `journal.events.insert(...)` are working Fjall write paths | Production path at `snapshots.rs:31-45` and the events keyspace; tested. |
| PO-003-PROPTEST | `prop::collection::vec(any::<u8>(), 0..=256)` is supported by proptest 1.5 | Proptest crate version pin; tool_metadata records `proptest@1.5`. |
| PO-004-LINT | `moon v2` is installed and on PATH; the lint-src task at `.moon/tasks/all.yml:46-62` is the canonical zero-tolerance source lint | AGENTS.md: "Moon v2 configuration is scaffolded in .moon/; moon ci remains the canonical gate." (environment contract) |

All assumptions are discharged by the Rust type checker or by existing
tests; none require a trust marker beyond `TB-CAP-001`.

---

## Self-audit against `references/plan-quality-gates.md` Gate 8 (Trust Marker Ledger)

| Check | Status |
|---|---|
| Obligation with `assumptions` non-empty has a `trusted_base_refs` entry per assumption | PASS (PO-001-UNIT and PO-001-REGRESSION have non-empty `assumptions` and `trusted_base_refs`; PO-002/003/004 have empty `assumptions` and empty `trusted_base_refs`; the only populated `trusted_base_refs` entry is `TB-CAP-001` which is documented above). |
| Obligation with `model_bounds` reducing a default bound has a `trusted_base_refs` entry explaining the reduction | PASS (no model-bound reductions; all `model_bounds` are recorded as the standard proptest `cases`/`input_size` budget). |
| Obligation with `kani::cover!` as property evidence has a `trusted_base_refs` entry | N/A (no Kani obligations in this plan). |
| Obligation with Flux `#[trusted]`/`#[ignore]`/`extern_spec`/`opaque` has a `trusted_base_refs` entry | N/A (no Flux obligations in this plan). |
| Miri obligation with `cfg_attr(miri, ignore)` skipped tests has a `trusted_base_refs` entry | N/A (no Miri obligations in this plan). |

END OF TRUSTED BASE PLAN.