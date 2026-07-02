# Hazard Analysis — vb-uwxct

This bead is TEST-ONLY. The hazards below are hazards to the **specimen
post-repair correctness**, not hazards to production behavior (production is
unchanged). Each hazard is named, classified, and tied to a mitigation or
proof seed.

## H1 — Specimen silently accepts the sentinel rejection

**Classification**: test-correctness / behavioral hazard
**Risk tags**: `parser/codec`, `public_api`
**Specimens**: all 7

**Description**: a repair that uses `Err(_) => prop_assert!(true)` or
`Err(_) => assert!(false)` erased suppresses the `SequenceOverflow` variant
without checking that the input was the sentinel. The specimen would then
silently pass when the encoder incorrectly rejected a non-sentinel value.

**Mitigation**:
- Preferred form: `seq in 0u64..u64::MAX` — does not see `Err` at all.
- Acceptable form: explicit match on `Err(JournalError::SequenceOverflow)` + sentinel check.
- Never use `Err(_) => prop_assert!(true)` without re-classifying.

## H2 — Specimen over-tightens and excludes valid inputs

**Classification**: spec drift / test-regression hazard
**Risk tags**: `parser/codec`
**Specimens**: all 7

**Description**: mis-placed `prop_assume!(s1 == 0)` or wrong range
`0..u64::MAX - 1` accidentally excludes valid encodable sequences and turns
the property into a vacuous assertion that always holds.

**Mitigation**:
- Prefer `in 0u64..u64::MAX` (half-open). Read the proptest boundary in
  `fjall_keyspace_manifest_tests.rs:129,131` as the canonical reference.
- Add a paired positive unit test (in `vb_storage/src/keys/tests.rs`) — NOT
  required for this bead but already exists at lines 484-505.

## H3 — Kani harness `kani::assume` masks a real bug

**Classification**: model-checked / vacuous-disproof hazard
**Risk tags**: `kani`, `E_KANI_ASSUMPTION_VACUITY`
**Specimens**: `assert_key_contracts`

**Description**: `kani::assume(seq_value != u64::MAX)` excludes the very input
that the encoder is documented to reject. If the encoder's rejection were
broken (e.g., starts rejecting when `seq > u64::MAX / 2`), the harness would
never witness the broken behavior.

**Mitigation**:
- Prefer the explicit `Err(JournalError::SequenceOverflow) => { assert!(seq_value == u64::MAX); }`
  match arm. This keeps the sentinel in-scope AND asserts the contract.
- The `kani::assume(seq_value != u64::MAX)` alternative is acceptable only
  if paired with a `cover!` claim that documents the bound.

## H4 — Specimen re-uses `Err(_)` for non-`SequenceOverflow` variants

**Classification**: test-correctness hazard
**Risk tags**: `public_api`
**Specimens**: all 7

**Description**: the encoder's only `Err` variant on the
`sequenced_run_key` path is `SequenceOverflow`. A specimen match arm that
treats any `Err(_)` as "expected rejection" would be silently wrong if a
future encoder change adds another `Err` variant.

**Mitigation**: enumerate `Err(JournalError::SequenceOverflow) => ...`
explicitly and keep an `Err(_) => prop_assert!(false, "unexpected Err variant")`
defensive arm. Or use the `in 0u64..u64::MAX` constraint that never sees `Err`.

## H5 — Property under test drifts to vacuous truth

**Classification**: behavior-affecting / `E_PROOF_SMOKE_MISSING` analogue
**Risk tags**: `parser/codec`
**Specimens**: all 6 proptests + harness assertions

**Description**: an over-tightened repair could weaken the property under
test (e.g., wrap the assertion in `if seq < u64::MAX { ... }` and end up
asserting nothing meaningful on the rest). Proof-theater shape.

**Mitigation**: the property statements in
`type-contracts.md` §4.1 must still hold verbatim on `seq ∈ 0..u64::MAX`.
The black-hat reviewer MUST walk through one full-case vector.

## H6 — Temporal / replay hazard at the production boundary

**Classification**: temporal / REQ-vb-om21-08
**Risk tags**: `temporal`, `persistence`
**Specimens**: harness + proptests near tail-scan

**Description**: sequence overflow semantics drive the tail-scan fallback
path; if a proptest accepts the wrong rejection shape, replay correctness
is silently weakened. The contract is exact:
`Err(JournalError::SequenceOverflow)` iff `seq == u64::MAX`.

**Mitigation**: the proof seeds in `proof-seeds.jsonl` reference
`REQ-vb-om21-08` (temporal). Replay correctness is asserted by the
sequence_numbering domain invariants — out of scope here but the contract
is reinforced.

## H7 — Proptest arithmetic subtle-bug hazard

**Classification**: Rust-core invariant
**Risk tags**: `parser/codec`
**Specimens**: `same_run_different_seq_keys_differ_in_seq_bytes` at 1427-1449

**Description**: `prop_assume!(run_val != 0 && s1 != s2)` is correct, but a
bug in the tightening could add `prop_assume!(s1 != u64::MAX && s2 != u64::MAX)`
which combined with `s1 != s2` accidentally forces BOTH sequences to satisfy
some test that no real `seq_value` would satisfy.

**Mitigation**: keep the existing `prop_assume!` clauses; only tighten the
proptest ranges or change the matcher.

## H8 — Doc-comment drift at `keys.rs:486-490`

**Classification**: doc-drift hazard
**Risk tags**: documentation
**Specimens**: none direct

**Description**: the canonical-positive test
`run_event_key_rejects_event_seq_max_sentinel` at
`vb_storage/src/keys/tests.rs:497-505` has a doc comment that describes the
current encoder implementation. The doc comment is correct and matches the
contract — no doc drift expected.

**Mitigation**: NONE for this bead — out of scope. Logged so a future bead
does not accidentally re-toggle the doc comment.

## H9 — Verifier mirror / production_inner drift

**Classification**: spec-mirror drift
**Risk tags**: `verification`
**Specimens**: none direct

**Description**: the Verus spec mirror at
`verification/verus/extern_vb_storage_keys.rs` and the production_inner
mirrors at `vb_vzcuf_PS_001_production.rs:144`,
`vb_vzcuf_PS_002_production.rs:179` already declare the
`SequenceOverflow` rejection. After this bead, no production code changes,
so the drift gate (`scripts/check-production-inner-drift.sh`) should remain
green.

**Mitigation**: out-of-scope here. The bead does not touch Verus, so the
mirror is preserved by construction.

## H10 — Forbidden `assert!(false)` / `.expect()` after repair

**Classification**: source lint
**Risk tags**: `source_lint`
**Specimens**: all 7

**Description**: the engineering rules forbid `panic`, `unwrap`, `expect`,
`todo`, `unimplemented`, `dbg`, and unchecked `assert!(false)` in
production code. Test code is more permissive but still prefers explicit
matchers. After the bead:

- Six proptests should NOT use `.expect()` on `run_event_key(...)`.
- One Kani harness should NOT use `Err(_) => assert!(false)` if the sentinel
  can occur.

**Mitigation**: the canonical pattern (`in 0u64..u64::MAX`) bypasses the
need for any `assert!(false)` and any `.expect()`. The Kani alternative is
explicit match on `SequenceOverflow`.

## H11 — Proptest cardinality blow-up under `in 0u64..u64::MAX`

**Classification**: test-runtime / performance hazard
**Risk tags**: runtime
**Specimens**: all 6 proptests

**Description**: changing `s1: u64, s2: u64` to `s1 in 0u64..u64::MAX, s2 in 0u64..u64::MAX`
shrinks the sampled space by one value per proptest, but the shrink factor
is negligible (`u64::MAX / u64::MAX == 1`). proptest still has full coverage.

**Mitigation**: confirm proptest default cases (256) still pass; this is
expected and acceptable.

## H12 — Kani harness now bounded by `seq` constraint, not `seq_raw`

**Classification**: model-precision hazard
**Risk tags**: `kani`
**Specimens**: `assert_key_contracts`

**Description**: the symbolic `seq_hi: u16, seq_lo: u16` packs into `seq_raw`
which spans `[0, 2^32)`. The post-repair contract treats `seq_value == u64::MAX`
as a separate case that may or may not be reachable (it is reachable iff
`seq_hi == 0xFFFF && seq_lo == 0xFFFF`, which IS within the symbolic range).

The harness tightening must NOT silently shift the contract. If
`kani::assume(seq_value != u64::MAX)` is chosen, the symbolic packing
loses one reachability point per proof run.

**Mitigation**: explicit `Err(JournalError::SequenceOverflow) => assert!(seq_value == u64::MAX)`
preserves full reachability and matches the production contract.
