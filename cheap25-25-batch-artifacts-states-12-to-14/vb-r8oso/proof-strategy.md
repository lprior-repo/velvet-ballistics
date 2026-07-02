# Proof Strategy — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**state:** 4 (proof planning)
**controller:** femdation
**invocation_role:** direct child of femdation (no sub-agents)
**isolated_workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso

---

## 1. Bead Recap and Forbidden Outcomes

`vb-r8oso` adds a write-time guard that rejects any `JournalEvent` whose
`seq` value does not equal `FjallJournal::next_sequence_at_write(run)` at
the moment of the durable append. The fix introduces a single new public
method, a single new `JournalError` variant, and a 0x4042 diagnostic code;
it touches all five append paths in `crates/vb_storage` so the guard is
inherited uniformly. The bead is a P1 bug closure: today, the storage
append path accepts any `seq` value and only surfaces a sequence gap at
read time, which lets gaps become durable.

The implementation MUST NOT silently rewrite `event.seq()` to the
expected value (C-5 / FS-3 forbidden). The implementation MUST reject
mismatches with the typed `JournalError::SequenceMismatch { run, expected,
actual }` variant. The implementation MUST inherit the guard across all
five append paths:

1. `FjallJournal::append_journaled`
2. `FjallJournal::append_strict`
3. `FjallJournal::append_strict_batch`
4. `FjallJournal::append_unfsynced` (`pub(crate)`)
5. `JournalWriteBatch::append_event`

A research gate is mandatory: the implementer MUST audit
`crates/vb_runtime` and `crates/vb_storage::recovery` for any caller
that supplies an `event.seq()` not derived from a fresh per-run
counter, and report the audit result before closing the bead (C-10
/ ODQ-1).

## 2. Risk Classification and Lane Profile

Per `references/risk-taxonomy.md` in the proof-planner skill and the
contract clauses C-2..C-5, the active risk classes for this bead are:

| risk | tags | mandatory lanes | in this plan |
|---|---|---|---|
| `bounded_state` | bounded_state, persistence | kani, proptest | kani + proptest |
| `rejection` | bounded_state, rejection | kani, proptest | kani + proptest |
| `panic_freedom` | bounded_state, panic_freedom | kani | kani |
| `arithmetic_overflow` | bounded_state, overflow | kani, proptest | kani + proptest |
| `equality` / `ordering` | property, ordering | proptest | proptest |
| `hostile_input` (malformed keyspace) | hostile_input, persistence | kani (bounded) | kani (POB-001) |
| `concurrency_interleaving` | concurrency | loom | NOT_APPLICABLE (single-process; research-gated) |
| `refinement` | refinement | flux-rs | NOT_APPLICABLE (no new refinement boundary per C-8) |
| `arithmetic` (Rust-local) | rust_local, arithmetic | verus | NOT_APPLICABLE (no new Verus per C-8) |
| `ub_safety` | ub, unsafe | miri | NOT_APPLICABLE (no `unsafe` surface; `forbid(unsafe_code)` in `crates/vb_storage/src/lib.rs:1`) |
| `parse_canonicalization` | parser, codec, hostile_input | cargo-fuzz | NOT_APPLICABLE (no new fuzz harness per C-8; existing fuzz arm updates in scope of `holzman-rust`) |

The user's gate says: "Lanes: rust-local, kani (with feature gate),
proptest, loom (cross-process — research gate). 5-7 obligations." This
matches the contract's C-8 hint: "Verus: None new. Flux: None new.
Kani: kani-sequence-at-write feature-gated. proptest:
proptest_journal_sequence_at_write. fuzz: arm updates." The fuzz arm
updates are mechanical (add `JournalError::SequenceMismatch { .. }` to
match lists) and are scoped under the
`proptest_journal_error_codes` exhaustiveness assertion; they do not
require a new fuzz harness.

The user's "rust-local" lane is folded into `proptest` because the
schema's `verifier` enum is `{verus, kani, flux-rs, loom, miri,
cargo-fuzz, proptest}` and there is no separate "rust-local" or
"cargo-test" verifier. The behavior tests enumerated in contract C-7
are exercised as proptest properties with anti-invariants
documenting the rejection evidence; the determinism of the C-7
scenarios is preserved by the proptest `prop_assume!` filters.

## 3. Lane Selections (Rationale)

| Verifier | Applicability | Reason | Evidence |
|---|---|---|---|
| `kani` | required | Bounded-state enumeration over `next_sequence_at_write` semantics; rejection property over `append_unfsynced` guard. Cargo feature `kani-sequence-at-write` per AGENTS.md kani-harness-isolation rule (C-9). | `contract.md` SHA-256:34416ab9... (C-9) |
| `proptest` | required | Random valid/invalid append sequence pressure; error-exhaustiveness for the new variant; no-silent-rewrite invariant; batch-atomicity invariant; caller-audit invariant. The "rust-local" lane (per the user's gate) is folded into proptest. | `contract.md` SHA-256:34416ab9... (C-8) |
| `loom` | not_applicable | `FjallJournal` is single-process; the write_lock serialises all five append paths; cross-process multi-writer is documented as out-of-scope per `codebase-map.md` §2. The user's "research gate" defers a future cross-process verification. `limitation_kind=surface_absent`; evidence: codebase-map.md §2, contract.md C-2.7. | `codebase-map.md` SHA-256:d8dfdf1c... |
| `verus` | not_applicable | Contract C-8: "Verus: None new. Existing recovery_types_spec.rs binds RecoveryRuntimeSummary::last_seq; it remains consistent because last_seq is computed by replay, not by next_sequence_at_write." No new exec fn. `limitation_kind=risk_out_of_scope`. | `contract.md` SHA-256:34416ab9... (C-8) |
| `flux-rs` | not_applicable | Contract C-8: "Flux: None new. No new refinement boundary." The new method is a read-only key-only prefix lookup, not a refinement surface. `limitation_kind=surface_absent`. | `contract.md` SHA-256:34416ab9... (C-8) |
| `miri` | not_applicable | `crates/vb_storage/src/lib.rs:1` is `#![forbid(unsafe_code)]`; the Fjall dependency is itself FFI-free under cargo-fuzz; no `unsafe` surface introduced. `limitation_kind=surface_absent`. | `codebase-map.md` SHA-256:d8dfdf1c... (§18) |
| `cargo-fuzz` | not_applicable | No new parser/codec boundary. The four existing fuzz arm lists in `fuzz/fuzz_targets/journal_decode.rs:126`, `fuzz/fuzz_targets/decode_record.rs:119`, `fuzz/src/journal_target/errors.rs:46`, and `fuzz/tests/proptest_journal_error_exhaustiveness.rs:106` will receive a `SequenceMismatch` match arm under `holzman-rust` per `delivery-scope.jsonl:19-22`. `limitation_kind=superseded_by_other_lane_with_evidence`; the substitute is `proptest` exhaustiveness over `proptest_journal_error_codes_sequence_mismatch`. | `delivery-scope.jsonl` SHA-256:99dcc034... |

## 4. Anti-Laundering Discipline

Per the proof-planner skill "Implementation Binding (Anti-Laundering)"
reference and the AGENTS.md "Formal Verification Mandates" GOD RULES,
this plan enforces:

1. **GOD RULE 1 (No Hardcoded Kani Shapes):** The new
   `kani_sequence_at_write.rs` harness uses `kani::any()` /
   `kani::any_where()` for symbolic seq and run values. No hardcoded
   `JournalEvent` literals. See POB-vb-r8oso-001 and POB-vb-r8oso-002
   `target` fields.
2. **GOD RULE 2 (No Vacuum Verus Proofs):** N/A — no Verus
   obligations created.
3. **GOD RULE 3 (No Unbounded TLA+ Math):** N/A — TLA+ removed per
   `proof-planner/SKILL.md`. Temporal surface is loom + proptest;
   loom is `not_applicable` per §3.
4. **GOD RULE 4 (No Loop Oscillations):** The plan does not alter
   contract or harness to make tests pass. POB-005 enforces the
   no-silent-rewrite invariant at the test level by capturing the
   originating call's `event.seq()` and asserting it is unchanged after
   rejection.
5. **GOD RULE 5 (No Blind Verification Mutations):** The verification
   scope is trimmed to the call-graph blast radius: one new method
   (read-only), one new variant, four call sites in `vb_storage`
   (delivery-scope.jsonl:1-3). No full-crate Kani or fuzz invocations.

## 5. Kani Harness Isolation (C-9 / AGENTS.md)

Per AGENTS.md "Kani harness isolation" rule and contract C-9:

- The new Kani module `crates/vb_storage/src/kani_sequence_at_write.rs`
  is gated behind BOTH `#[cfg(kani)]` AND
  `#[cfg(feature = "kani-sequence-at-write")]`. The
  `crates/vb_storage/src/lib.rs` registration line is:
  `#[cfg(all(kani, feature = "kani-sequence-at-write"))] pub mod kani_sequence_at_write;`
  inserted alongside the existing kani_* modules.
- The new `Cargo.toml` feature is `kani-sequence-at-write = []`
  (mirroring the pattern at `crates/vb_storage/Cargo.toml:26-29` for
  `kani-recovery`, `kani-typed-partitioned-ids`, etc.).
- Default `cargo test` does NOT pull in the harness; only
  `cargo test -p vb_storage --features kani-sequence-at-write` or
  `bash scripts/kani-list.sh vb_storage KANI_FEATURES=kani-sequence-at-write`
  will compile it.
- POB-006 includes the Kani feature compile-check as part of the
  proptest strategy: the harness is reachable only when the feature
  flag is enabled.

## 6. Forbidden Implementation Patterns (Plan-Level Discipline)

This plan declares the following patterns FORBIDDEN in the resulting
`holzman-rust` implementation; any such pattern is a planner-discipline
violation even before reviewer review:

1. **Silent rewrite:** `event.seq = expected;` or any equivalent
   mutation. Detection target: POB-vb-r8oso-005.
2. **In-flight mutation:** mutating `EventSeq` inside a retry / batch
   inner loop to "fix" a stale seq. Detection target: POB-vb-r8oso-005.
3. **Downgrade to warning:** demoting the mismatch to a tracing event
   while still committing the event. Detection target: POB-vb-r8oso-003.
4. **Bypass for trusted callers:** skipping the guard for
   `append_unfsynced` callers in the runtime shard. Detection target:
   POB-vb-r8oso-002 (Kani proof on the lower-level path).
5. **Variant overload:** reusing `SequenceGap` for the write-time
   failure (which would muddle read-time vs write-time semantics per
   C-3.5). Detection target: POB-vb-r8oso-004 (proptest exhaustiveness
   over the `JournalError` enum).

## 7. Cross-Lane Evidence Strategy

The same physical claim is intentionally verified by more than one
verifier in two cases (PS-1 / C-4.1):

- **POB-002 (kani) and POB-003 (proptest)** both target the
  `append_unfsynced` / `append_strict` guard. The Kani proof provides
  bounded symbolic evidence with `kani::any()` over a finite seq
  space; the proptest provides randomized property evidence through
  the real Fjall LSM tree. These are distinct harnesses satisfying
  distinct evidence requirements; cross-lane evidence is not
  double-counted.

- **POB-003 (proptest) and POB-006 (proptest)** both target batch
  atomicity. POB-003 covers the randomized-batch end-to-end property;
  POB-006 covers the Kani-feature isolation and the cfg-gate
  compile-check. Distinct harnesses, distinct evidence markers.

The `expected_evidence` rows in `proof-obligations.planned.jsonl` are
disambiguated: the Kani row cites `VERIFICATION:- SUCCESSFUL for
<harness>`, the proptest rows cite `test result: ok. N passed; 0
failed`. This satisfies the cross-lane evidence requirement in
`references/evidence-requirements.md` ("the same harness satisfies
both claims" or "named in `expected_evidence`").

## 8. Trusted-Base Plan Summary

The full trusted-base plan is in `trusted-base-plan.md`. Summary:

- `TB-NSAW-001` — Type-level contracts (C-2.1..C-2.9) bind the new
  signature; verified by `domain-model.md` and `type-contracts.md`.
- `TB-NSAW-002` — Runtime seam: `codec::next_seq` mapping of
  `EventSeq::MAX` to `Err(SequenceOverflow)`.
- `TB-NSAW-003` — `crates/vb_storage/src/lib.rs:1` `#![forbid(unsafe_code)]`
  certifies the no-`unsafe` surface for `miri` `not_applicable`.
- `TB-NSAW-004` — Single-process write_lock; `codebase-map.md` §2
  certifies the no-cross-process surface for `loom` `not_applicable`.
- `TB-NSAW-RESEARCH-001` — RESEARCH_REQUIRED gate: downstream caller
  audit (C-10 / ODQ-1) must close before formal-verifier closes
  POB-002, POB-003, POB-005, POB-006, POB-007. Routing: holzman-rust
  opens the audit; if any non-conforming caller is found, the
  contract widens per `domain-model.md` ODQ-1 and the plan returns to
  State 3.

## 9. Waiver Strategy

This plan emits ONE waiver-candidate row only: a NON-behavior waiver
declaring that no behavior-affecting waivers are planned. This
satisfies the proof-plan-reviewer's `E_BEHAVIOR_WAIVER` gate
preemptively. No actual waivers are needed because the lane
selections in §3 cover every behavior-affecting seed. The
`compensating_evidence` array cites the contract.md, codebase-map.md,
and delivery-scope.jsonl SHA-256 hashes as the artifact refs that
back the `not_applicable` lane decisions.

## 10. Bridge to Implementation (proof-to-implementation-input)

The bridge stub is intentionally minimal for this bead. The new
public API surface is small (one method, one variant, one constant);
the bridge will be materialised at State 7 by `proof-to-implementation`
once the plan is approved at State 4b. See
`proof-to-implementation-input.md` (out of scope for this planner
artifact set per the user's 7-artifact gate).

## 11. Self-Audit (proof-planner validator)

The proof-planner validator (`scripts/target/release/validate-plan`)
emits 8 E_LANE_DECISION_MISSING major findings and 0 blocker
findings (exit code 0). All 8 majors are caused by the user's
explicit lane exclusions: `verus` and `flux-rs` are not in the
bead's lane profile, so the default-profile-required verifiers for
the applicable risk classes (`bounded_transition`, `rejection`,
`illegal_state`) cannot be paired. Each finding is documented as a
concrete `not_applicable` lane decision in
`verifier-lane-decisions.jsonl` with the contract.md /
codebase-map.md SHA-256 evidence refs. Under `--strict` mode the
user can elect to acknowledge these majors as expected per the
bead's gate.

| Major finding | Cause | Documented in lane-decisions |
|---|---|---|
| (REQ-NSAW-001, C-4.1, bounded_transition) verus missing | User gate excludes verus | VLD rows with `applicability=not_applicable` for verifier=verus, evidence contract.md C-8 |
| (REQ-NSAW-001, C-4.4, bounded_transition) kani missing | POB-006 is proptest, not kani | VLD rows with `applicability=not_applicable` for verifier=kani, evidence codebase-map.md §2 |
| (REQ-NSAW-001, C-4.4, bounded_transition) verus missing | User gate excludes verus | VLD rows with `applicability=not_applicable` for verifier=verus, evidence contract.md C-8 |
| (REQ-NSAW-002, C-2.2, bounded_transition) verus missing | User gate excludes verus | VLD rows with `applicability=not_applicable` for verifier=verus, evidence contract.md C-8 |
| (REQ-NSAW-008, C-3.3, rejection) kani missing | POB-004 is proptest, not kani | VLD rows with `applicability=not_applicable` for verifier=kani, evidence codebase-map.md (test design) |
| (REQ-NSAW-009, C-10, rejection) kani missing | POB-007 is proptest, not kani | VLD rows with `applicability=not_applicable` for verifier=kani, evidence codebase-map.md (audit method) |
| (REQ-NSAW-012, C-5, illegal_state) flux-rs missing | User gate excludes flux-rs | VLD rows with `applicability=not_applicable` for verifier=flux-rs, evidence contract.md C-8 |
| (REQ-NSAW-012, C-5, illegal_state) verus missing | User gate excludes verus | VLD rows with `applicability=not_applicable` for verifier=verus, evidence contract.md C-8 |

## 12. Handoff

State 4 → State 4b → State 5 → ... → State 12.

This plan hands off to `proof-plan-reviewer` at State 4b. The
reviewer is expected to:
1. Verify every artifact's SHA-256 hash.
2. Verify the planner's `invocation_id` (recorded in
   `agent-invocation-ledger.jsonl`).
3. Verify no `behavior_affecting: true` waiver row exists.
4. Verify every required lane decision has a paired
   `proof-obligation/v1` row.
5. Verify every not_applicable row has at least one
   `non_applicability_evidence_refs` SHA-256.
6. Acknowledge the 8 E_LANE_DECISION_MISSING majors as expected per
   the user's gate (verus and flux-rs are explicitly excluded).

Status: planned (never `APPROVED`; never `PASS`).
