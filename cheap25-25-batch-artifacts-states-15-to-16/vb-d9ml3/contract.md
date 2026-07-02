# Contract — Storage Trim/Snapshot Key Length Cap (vb-d9ml3)

## Bead ID

`vb-d9ml3` — Storage: reject overlong malformed trim and snapshot keys (P1 bug)

## Bead Scope (verbatim)

> Storage key parsing accepts trim/snapshot keys of unbounded length. Add a
> length cap (e.g., `MAX_TRIM_KEY_LEN`, `MAX_SNAPSHOT_KEY_LEN`) and reject
> overlong keys with a typed error.

## Contract Lane

**Rust-local implementation** — numeric/cap refinement, parser/codec,
persistence, public-API stability. **No formal verifier required**
(Kani/Verus/Flux/Loom/fuzz lanes explicitly out of scope per
`delivery-scope.jsonl` rows 35-39).

---

## Pre-conditions (must hold at contract acceptance)

1. `crates/vb_storage/src/constants.rs:74` currently declares
   `pub(crate) const JOURNAL_KEY_BYTES: usize = 17;` as the source of
   truth for journal key envelope byte length.
2. `crates/vb_storage/src/trimming/logic.rs:36, 77, 222` currently use
   the magic literal `17` for the length check at three trim call sites.
3. `TrimError::IncompleteTrim { deleted_count: u64 }` is a public variant
   with diagnostic code `0x4102` (`trimming/mod.rs:51-54, 62`).
4. `JournalError::Trim(Box<TrimError>)` wraps `TrimError` and delegates
   `diagnostic_code()` to the inner error (`error/mod.rs:187`,
   `error/codes.rs:167`).
5. `crates/vb_storage/src/snapshot_tests.rs:208-248` (Round 10 issue 7
   regression) and `crates/vb_storage/src/trimming/tests.rs:875-987`
   (SC-006 / CC-002 fail-closed tests) currently pass.

---

## Contract Clauses

Each clause has an ID, a domain claim, an implementation site, an invariant
binding, and a test-pinning reference.

### CC-CAP-001 — Named-Cap Aliases

**Claim**: Two new `pub(crate) const` aliases are declared in
`crates/vb_storage/src/constants.rs`, co-located with `JOURNAL_KEY_BYTES`:

```rust
pub(crate) const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;
pub(crate) const MAX_SNAPSHOT_KEY_LEN: usize = JOURNAL_KEY_BYTES;
```

**Invariant**: `MAX_TRIM_KEY_LEN == MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES == 17`.
This is a compile-time invariant enforced by the `const` alias chain.

**Forbidden**: defining either alias as a literal `17` at the alias site.
The alias MUST be a `const` reference to `JOURNAL_KEY_BYTES`.

**Doc obligation**: a doc-comment block on the new aliases explains the
journal key envelope `[prefix:u8][run_id:u64 BE][seq:u64 BE]` and the
domain meaning of the cap.

**Test pinning**: `cargo check` (compile-time); a unit test that asserts
`assert_eq!(MAX_TRIM_KEY_LEN, 17)` and
`assert_eq!(MAX_SNAPSHOT_KEY_LEN, 17)`.

---

### CC-CAP-002 — Replace Magic `17` at `logic.rs:36`

**Claim**: The length check at `trimming/logic.rs:36` is rewritten from
`if key.len() != 17` to `if key.len() != MAX_SNAPSHOT_KEY_LEN`.

**Invariant**: the check uses the named cap, not a magic literal.

**Bound**: this site is the snapshot keyspace length check (single line
within `latest_durable_snapshot_seq`).

**Test pinning**: `snapshot_tests.rs:208-248` (existing Round 10 issue 7
regression, asserting `IncompleteTrim { deleted_count: 0 }` for a 13-byte
planted key) PLUS a new test case in the same file that plants an
overlong key (e.g., 24 bytes) under `PREFIX_RUN_SNAPSHOT` and asserts the
same `IncompleteTrim { deleted_count: 0 }` error.

---

### CC-CAP-003 — Replace Magic `17` at `logic.rs:77`

**Claim**: The length check at `trimming/logic.rs:77` is rewritten from
`if key.len() != 17` to `if key.len() != MAX_TRIM_KEY_LEN`.

**Invariant**: the check uses the named cap.

**Bound**: this site is the trim-loop length check within
`trim_events_for_run`.

**Test pinning**: `trimming/tests.rs:875-932` (existing SC-006 fail-closed
test, planting a 9-byte short key under `PREFIX_RUN_EVENT`) PLUS a new
test case that plants an overlong key (e.g., 24 bytes) under the same
prefix and asserts `IncompleteTrim { .. }`.

---

### CC-CAP-004 — Replace Magic `17` at `logic.rs:222`

**Claim**: The length check at `trimming/logic.rs:222` is rewritten from
`if key.len() != 17` to `if key.len() != MAX_TRIM_KEY_LEN`.

**Invariant**: the check uses the named cap.

**Bound**: this site is the diagnostic-count length check within
`count_trimmable_events`.

**Test pinning**: `trimming/tests.rs:934-987` (existing CC-002 fail-closed
test, planting a 9-byte short key under `PREFIX_RUN_EVENT`) PLUS a new
test case that plants an overlong key (e.g., 24 bytes) under the same
prefix and asserts `JournalError::Trim(IncompleteTrim { .. })`.

---

### CC-CAP-005 — Typed-Error Shape Preservation

**Claim**: The trim scanners surface a typed error of variant
`TrimError::IncompleteTrim { deleted_count: u64 }` for any non-canonical
raw key. Diagnostic code `0x4102` is preserved.

**Invariant**:

1. `TrimError::IncompleteTrim` shape is unchanged (single field
   `deleted_count: u64`).
2. `TrimError::INCOMPLETE_TRIM_CODE == 0x4102` is unchanged.
3. `TrimError::diagnostic_code()` returns `0x4102` for the
   `IncompleteTrim` arm (`trimming/mod.rs:71`).
4. `JournalError::Trim(inner).diagnostic_code()` returns
   `inner.diagnostic_code()` (`error/codes.rs:167`).

**Forbidden**:

- Introducing a new `TrimError` variant for overlong keys.
- Converging on `JournalError::MalformedKeyspaceRow` for the trim path
  (different code `0x4030`, different shape, would break existing tests).
- Adding a new diagnostic code (`0x4104`, etc.) for overlong keys.

**Test pinning**: `error_code_tests.rs:~244` (existing propagation test);
`snapshot_tests.rs:235` (structural `IncompleteTrim { deleted_count: 0 }`
assertion); `trimming/tests.rs:929, 984` (structural `IncompleteTrim { .. }`
assertion).

---

### CC-CAP-006 — Fail-Closed Workflow

**Claim**: All three trim scanners abort on the first non-canonical
observation with the typed error; no skip-and-continue, no truncation,
no padding.

**Invariant**: the scanner returns `Err(TrimError::IncompleteTrim { .. })`
on the first raw key whose `key.len() != MAX_*_KEY_LEN`. No subsequent
keys are inspected. The LSM batch is not committed.

**Test pinning**: the existing `snapshot_tests.rs:208-248` and
`trimming/tests.rs:875-987` tests already plant a non-canonical key
BEFORE a valid key (or as the only key) and assert the abort; the new
overlong cases follow the same pattern.

---

### CC-CAP-007 — Counter Progress Preservation

**Claim**: When the trim scanner aborts, the `deleted_count` field
preserves the partial progress count (number of events already removed
from the LSM batch).

**Invariant**:

- `trim_events_for_run`: abort returns
  `IncompleteTrim { deleted_count }` where `deleted_count` is the
  saturating count of events already removed.
- `count_trimmable_events`: abort returns
  `JournalError::Trim(Box::new(TrimError::IncompleteTrim { deleted_count: count }))`
  where `count` is the saturating count of events already counted.
- `latest_durable_snapshot_seq`: abort returns
  `IncompleteTrim { deleted_count: 0 }` (the snapshot lookup never
  accumulates a counter).

**Test pinning**: existing tests `trimming/tests.rs:929, 984` use
`IncompleteTrim { .. }` (matches any counter value); the new tests
can additionally pin a specific `deleted_count` if the planner wants
stronger assertion.

---

### CC-CAP-008 — No Cross-Crate Change

**Claim**: The implementation makes zero changes outside `vb_storage`.

**Invariant**: `crates/vb_core`, `crates/vb_runtime`, `crates/vb_cli`,
`crates/vb_validate` are unchanged.

**Test pinning**: `cargo check --workspace` continues to pass with the
new aliases visible only to `vb_storage`.

---

### CC-CAP-009 — Existing Tests Continue to Pass

**Claim**: All existing tests at `snapshot_tests.rs:208-248` and
`trimming/tests.rs:875-987` pass post-fix without modification of their
assertion structure.

**Invariant**: the structural `Err(TrimError::IncompleteTrim { deleted_count: 0 })`
assertion at `snapshot_tests.rs:235` is preserved.

**Test pinning**: `cargo test -p vb_storage --lib snapshot_tests`
and `cargo test -p vb_storage --lib trimming::tests` are GREEN.

---

### CC-CAP-010 — New Overlong Test Cases Added

**Claim**: Three new test cases are added (one per magic-17 site) that
plant an overlong raw key (length > 17) under the appropriate prefix and
assert the typed error.

**Invariant**: each new test plants a raw key whose length is strictly
greater than `JOURNAL_KEY_BYTES` (e.g., 24 bytes) and asserts the same
typed error that the existing short-key tests assert.

**Test pinning**: the new test cases are co-located with the existing
tests (no new test file is needed):

| New test name | File | Length planted |
|---------------|------|----------------|
| `latest_durable_snapshot_seq_rejects_overlong_snapshot_key` (or similar) | `snapshot_tests.rs:~248` | e.g., 24 |
| `trim_events_for_run_fails_closed_on_overlong_event_key` | `trimming/tests.rs:~932` | e.g., 24 |
| `trim_eligibility_diagnostic_fails_closed_on_overlong_event_key` | `trimming/tests.rs:~987` | e.g., 24 |

---

## Clause-to-Code Mapping

| Clause | Production site | Test pinning site |
|--------|-----------------|--------------------|
| CC-CAP-001 | `constants.rs` (new aliases after line 74) | compile-time + new unit test |
| CC-CAP-002 | `trimming/logic.rs:36` | `snapshot_tests.rs:208-248` + augmented overlong case |
| CC-CAP-003 | `trimming/logic.rs:77` | `trimming/tests.rs:875-932` + augmented overlong case |
| CC-CAP-004 | `trimming/logic.rs:222` | `trimming/tests.rs:934-987` + augmented overlong case |
| CC-CAP-005 | `trimming/mod.rs:51-54, 62, 71` + `error/codes.rs:167` | `error_code_tests.rs:~244` |
| CC-CAP-006 | `trimming/logic.rs:36-38, 77-79, 222-226` | existing + augmented tests |
| CC-CAP-007 | `trimming/logic.rs:78, 82, 85, 223, 228, 232` | existing + augmented tests |
| CC-CAP-008 | n/a (constraint only) | `cargo check --workspace` |
| CC-CAP-009 | n/a (preservation constraint) | `cargo test -p vb_storage` |
| CC-CAP-010 | test files | co-located test additions |

---

## Verifier Lane Profile (for the proof planner)

Per `delivery-scope.jsonl` rows 32-39:

| Lane | Status | Rationale |
|------|--------|-----------|
| `default_rust_lane` (unit tests) | **REQUIRED** | CC-CAP-001 unit test, augmented cap tests |
| `proptest` | **REQUIRED** | CC-CAP-002/003/004: arbitrary-length key generator |
| `integration` | **REQUIRED** | CC-CAP-002/003/004: existing temp_journal-backed tests |
| `kani` | **NOT_REQUIRED** | Pure numeric/cap refinement; const alias chain is compile-time |
| `verus` | **NOT_REQUIRED** | No new `exec fn` with a non-trivial bound |
| `flux` | **NOT_REQUIRED** | Pure length refinement on a `usize` value already discharged by const |
| `fuzz` | **NOT_REQUIRED** | Encoders are pure 1-input=1-output `ArrayVec` writes; fuzz adds no coverage beyond proptest |
| `looom` (loom) | **NOT_REQUIRED** | No concurrent state transition surface |
| `default_rust_lane` (additional) | **REQUIRED** | `cargo check --workspace` for CC-CAP-008 |

The proof planner's `proof-obligations.planned.jsonl` should plan:

1. One unit test for CC-CAP-001 (const equality).
2. Three integration tests for CC-CAP-002/003/004 (existing + augmented).
3. One proptest for the cap-enforcement surface (arbitrary key lengths).
4. One regression test for CC-CAP-005 (`error_code_tests.rs` continues to pass).
5. Five `not_applicable` rows for Kani/Verus/Flux/Fuzz/Loom with the rationale
   documented in `delivery-scope.jsonl` rows 35-39.

---

## Open Domain Questions Resolved by This Contract

1. **Re-use `JOURNAL_KEY_BYTES` or alias?** → **Add the two new aliases
   (CC-CAP-001)**. The bead text explicitly names
   `MAX_TRIM_KEY_LEN, MAX_SNAPSHOT_KEY_LEN`.
2. **Typed-error target?** → **Reuse `TrimError::IncompleteTrim` (CC-CAP-005)**.
   The existing structural test assertions at `snapshot_tests.rs:235` and
   `trimming/tests.rs:929, 984` pin this variant; converging on
   `MalformedKeyspaceRow` would break those tests.
3. **Truncate vs. reject?** → **Reject (fail-closed) (CC-CAP-006)**.
   The existing behavior is reject; the contract preserves it.

## Open Questions Deferred to the Proof Planner

1. **Property test shape**: does the planner want a single proptest that
   enumerates lengths `0..=256` for both prefixes, or three separate
   proptests (one per call site)? The contract does not commit; the
   planner should choose based on the existing `keys/tests.rs` patterns.
2. **New test naming**: the contract suggests test names but does not
   commit. The planner / test-writer may choose any name that surfaces
   the overlong-key surface.

---

## Risk-Profile Summary

| Aspect | Status |
|--------|--------|
| Public API change | **None** (alias chain + variant preservation) |
| Diagnostic code change | **None** (`0x4102` preserved) |
| Cross-crate change | **None** |
| Existing test breakage | **None** (all structural assertions preserved) |
| New test surface | 3 new tests + 1 unit test + 1 proptest |
| Verifier lanes | 4 required + 5 not-required (with rationale) |

The bead is a **low-blast-radius** internal fix with a **bounded
verification surface**. The contract commits to:

- Adding two `pub(crate) const` aliases (CC-CAP-001).
- Replacing three magic literals with the named caps (CC-CAP-002/003/004).
- Preserving the typed-error variant + code (CC-CAP-005).
- Preserving the fail-closed workflow (CC-CAP-006).
- Preserving the counter semantics (CC-CAP-007).
- Adding three overlong test cases (CC-CAP-010).
- Zero cross-crate changes (CC-CAP-008).
- Zero existing test breakage (CC-CAP-009).

END OF CONTRACT.