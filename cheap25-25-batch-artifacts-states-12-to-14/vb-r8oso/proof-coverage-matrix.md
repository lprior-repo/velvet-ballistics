# Proof Coverage Matrix — vb-r8oso

This matrix maps every `proof-obligation/v1` row in
`proof-obligations.planned.jsonl` to (a) the contract clause it
satisfies, (b) the behavior-affecting flag, and (c) the proof seeds
the obligation covers. The matrix is the canonical cross-reference
the reviewer uses to confirm no seed is dropped and no clause is
uncovered.

## Obligation Index

| POB ID | Verifier | Contract clauses | Behavior-affecting | Proof seeds covered |
|---|---|---|---|---|
| POB-vb-r8oso-001 | kani | C-2.2, C-2.4, C-2.5 | true | PS-4, PS-6, PS-7, PS-9 |
| POB-vb-r8oso-002 | kani | C-4.1 | true | PS-1, PS-2, PS-3, PS-14 |
| POB-vb-r8oso-003 | proptest | C-2.2, C-2.4, C-2.5, C-4.1, C-4.4, C-5 | true | PS-1, PS-2, PS-3, PS-4, PS-6, PS-7, PS-8, PS-9, PS-14 |
| POB-vb-r8oso-004 | proptest | C-3.2, C-3.3, C-3.5 | true | PS-5, PS-11, PS-16 |
| POB-vb-r8oso-005 | proptest | C-3.2, C-5, C-6.1, C-6.2 | true | PS-5, PS-15 |
| POB-vb-r8oso-006 | proptest | C-4.4, C-9 | true | PS-3, PS-13, PS-14 |
| POB-vb-r8oso-007 | proptest | C-10 | true | PS-12 |

Total: 7 obligations, all `behavior_affecting: true`, covering
16/16 proof seeds.

## Clause Coverage

| Contract clause | POB | Notes |
|---|---|---|
| C-1.1 (ubiquitous language) | (n/a; language is enforced by naming, not proof) | Contract-level; verified by `cargo doc` and code review. |
| C-1.2 (FS-1 forbidden state) | POB-001, POB-002, POB-003, POB-006 | Forbidden state is a meta-claim that the union of "guard fires + batch atomic + error exhaustiveness" makes the gap-during-this-process unobservable. |
| C-1.3 (FS-3 forbidden state) | POB-002, POB-003, POB-005 | No-silent-rewrite. |
| C-2.1 (signature) | POB-001, POB-002 | Both call sites use the same `pub fn next_sequence_at_write(&self, run: RunId) -> Result<EventSeq, JournalError>` signature. |
| C-2.2 (return ZERO for fresh) | POB-001, POB-003 | kani covers prefix-iterator empty case; proptest covers randomized empty case. |
| C-2.3 (return succ for non-empty) | POB-001, POB-003 | kani covers prefix-iterator non-empty case; proptest covers randomized non-empty case. |
| C-2.4 (key-only lookup) | POB-001, POB-003 | kani harness asserts no value decode; proptest asserts the no-decode property via fuzz-injection of a poisoned value. |
| C-2.5 (overflow) | POB-001, POB-003 | kani covers u64::MAX succ boundary; proptest covers randomized boundary. |
| C-2.6 (RunId::ZERO) | POB-001 | kani harness asserts InvalidRunId. |
| C-2.7 (locking) | POB-003 | proptest asserts the function does not acquire write_lock (compile-time check + behavior-test). |
| C-2.8 (public wrapper) | POB-003 | proptest exercises `public_api::next_sequence_at_write`. |
| C-2.9 (no panic) | POB-001, POB-003 | kani asserts no panic on every path; proptest asserts no panic across 10000 cases. |
| C-3.1 (variant declaration) | POB-004, POB-005 | proptest exhaustiveness + cargo-test field-shape assertion. |
| C-3.2 (constructor pre-condition) | POB-004, POB-005 | proptest asserts `expected != actual`; cargo-test asserts constructor returns Err-equivalent state if violated. |
| C-3.3 (diagnostic code 0x4042) | POB-004 | proptest asserts `diagnostic_code == 0x4042` and `symbolic_code == "JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"`. |
| C-3.4 (code-registry) | POB-004 | proptest asserts the symbolic code is in CODE_REGISTRY (or the INTERNAL_INVARIANT fallback is acceptable per C-3.4). |
| C-3.5 (coexistence with SequenceGap) | POB-004 | proptest asserts `SequenceGap` is never returned by an append path; `SequenceMismatch` is never returned by `events_for_run`. |
| C-4.1 (uniform post-condition) | POB-002, POB-003 | All five append paths inherit the guard. |
| C-4.2 (guard precedence slot 3) | POB-002 | kani harness asserts the new guard sits at slot 3 (between `event.is_valid()` and the same-batch duplicate check). |
| C-4.3 (doc-comment growth) | (n/a; documentation) | holzman-rust updates doc-comments. |
| C-4.4 (batch atomicity) | POB-002, POB-003, POB-006 | kani covers single-batch atomicity; proptest covers randomized batch atomicity; cargo-test covers the C-7 test `append_strict_batch_rejects_on_first_mismatch_atomically`. |
| C-5 (no silent rewrite) | POB-003, POB-005 | Both proptest lanes cover the no-silent-rewrite invariant. |
| C-6.1 (existing test #1737 update) | POB-005 | proptest reclassification: append_strict(seq=2) after seq=0 returns `Err(SequenceMismatch { expected: 1, actual: 2 })`. |
| C-6.2 (existing test #4612 update) | POB-005 | proptest reclassification: append_journaled(seq=5) after seq=0 returns `Err(SequenceMismatch { expected: 1, actual: 5 })`. |
| C-6.3 (existing test #4585 update) | POB-005 | proptest reclassification: the test must rename or split. |
| C-6.4 (test-planner choice) | POB-005 | test-planner records the choice in the test-plan; holzman-rust implements per choice. |
| C-7 (test seeds) | POB-003, POB-005 | All seven test seeds enumerated. |
| C-8 (lane hints) | (n/a; lane hints only) | Lane decisions are the planner's output; C-8 is the input. |
| C-9 (Kani harness isolation) | POB-006 | proptest asserts the new module is gated behind `cfg(all(kani, feature = "kani-sequence-at-write"))`. |
| C-10 (downstream caller audit) | POB-007 | proptest asserts the audit report exists and lists every called site. |
| C-11 (acceptance gate) | (n/a; the gate is executed at State 12) | `moon ci` is the canonical final gate; the POBs above are the unit-level gates. |
| C-12 (cross-stage handoff) | (n/a; handoff is the planner's output) | This plan is the handoff. |

## Behavior-Affecting Index

| POB ID | behavior_affecting | Required? | Notes |
|---|---|---|---|
| POB-vb-r8oso-001 | true | true | New method semantics; gate is kani harness + feature isolation. |
| POB-vb-r8oso-002 | true | true | New guard fires; gate is kani bounded enumeration. |
| POB-vb-r8oso-003 | true | true | Random append sequence contiguity. |
| POB-vb-r8oso-004 | true | true | Error variant exhaustiveness. |
| POB-vb-r8oso-005 | true | true | No-silent-rewrite. |
| POB-vb-r8oso-006 | true | true | Batch atomicity + Kani feature isolation. |
| POB-vb-r8oso-007 | true | true | Downstream caller audit. |

No behavior-affecting waiver is planned. See `waiver-candidates.jsonl`.

## Cross-Lane Evidence Disambiguation

The same physical claim is verified by more than one verifier for
the following POBs:

- POB-002 (kani) and POB-003 (proptest) both target the
  `append_unfsynced` / `append_strict` guard. The kani proof
  harness and the proptest name distinct harnesses; their
  `expected_evidence` strings name different success markers
  (`VERIFICATION:- SUCCESSFUL` vs `test result: ok`). This is the
  cross-lane evidence pattern documented in
  `references/evidence-requirements.md` §"Multi-Lane Evidence".

- POB-003 (proptest) and POB-006 (proptest) both target batch
  atomicity. POB-003 covers the randomized-batch end-to-end property;
  POB-006 covers the Kani-feature isolation and the cfg-gate
  compile-check. Distinct harnesses, distinct evidence markers.

## Risk-Profile Audit

Each POB's `risk` field is checked against `risk-taxonomy.md`:

| POB | risk | matched tag(s) |
|---|---|---|
| POB-001 | bounded_transition | bounded_state |
| POB-002 | rejection | rejection |
| POB-003 | bounded_transition | bounded_state, property |
| POB-004 | rejection | rejection, public_api |
| POB-005 | illegal_state | rejection |
| POB-006 | bounded_transition | bounded_state |
| POB-007 | rejection | rejection, behavior_affecting |
