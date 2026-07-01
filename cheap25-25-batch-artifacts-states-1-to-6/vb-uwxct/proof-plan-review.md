reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: vb-uwxct-state4-proof-plan-review-attempt1
planner_invocation_id: vb-uwxct-state4-proof-planner-attempt1
review_state: 4
reviewed_at: 2026-07-01T16:35:00Z
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct
jj_workspace: cheap25-vb-uwxct
bead: vb-uwxct

# Proof Plan Review: vb-uwxct

## Review Metadata

- **bead**: `vb-uwxct`
- **title**: Tests: make max-sequence/key tests reject only exact overflow (P1 bug)
- **kind**: TEST-ONLY REPAIR
- **scope**: tighten six over-rejecting proptests in
  `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1326-1449`
  to use `0u64..u64::MAX`; replace `Err(_) => assert!(false)` in the Kani harness
  `vb_eepg_typed_partitioned_ids` / `assert_key_contracts` at
  `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-115` with an explicit
  match on `Err(JournalError::SequenceOverflow)`.
- **production surface**: NONE (production `keys.rs:480-496` is reference-only).

## Reviewed Artifacts

| Artifact | Hash (sha256) | Status |
|----------|---------------|--------|
| `contract.md` | `f670a6e6a41523253851ce94b668e75a92b37cfa56b463f388d7ff1a128b6f0b` | reviewed |
| `proof-seeds.jsonl` (8 rows) | `08bc408dca6a7d316889d7abf3dea5130f21645b12555bada5a19318ba0fe6d9` | reviewed |
| `proof-strategy.md` | `942aa5d8234c4558ffc215407e0a126d0f21fb09afea5db4182846af183f113c` | reviewed |
| `verifier-lane-decisions.jsonl` (64 rows) | `b13ce5664de5fc161118d33121222d62f045b4d24bc22b4490bf944b29db52ed` | reviewed |
| `proof-obligations.planned.jsonl` (4 rows) | `f164e741f29cdb241b7a4cc7c16727a4def43d99d93c5c78d3acd22efc0c6165` | reviewed |
| `trusted-base-plan.md` (4 entries) | `bc25fe13b44c3c7dd3a479b387b72383f16f6c5f727afeff1efac9edca868390` | reviewed |
| `waiver-candidates.jsonl` (6 rows) | `7b08e60c7066cb2b627f378fa9a763e41c7aa6e994202d88287a0d7ec56ca15a` | reviewed |
| `verifier-lane-review.jsonl` (64 rows) | `46e3cbc693007543e25fc9f526dad08c9662f5d2cfaf20c022f3a27f493b3246` | reviewed (this artifact) |

Production source files cross-referenced by the plan and verified at review time:

| Path | Lines | Verified content |
|------|-------|------------------|
| `crates/vb_storage/src/keys.rs` | 81-83 | `pub fn run_event_key(run: RunId, seq: EventSeq) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError>` delegating to `journal_key` |
| `crates/vb_storage/src/keys.rs` | 476-478 | `journal_key` calls `sequenced_run_key(PREFIX_RUN_EVENT, run, seq)` |
| `crates/vb_storage/src/keys.rs` | 480-496 | `sequenced_run_key` returns `Err(JournalError::SequenceOverflow)` iff `seq.get() == u64::MAX`; Ok `[u8; JOURNAL_KEY_BYTES]` otherwise |
| `crates/vb_storage/src/kani_typed_partitioned_ids.rs` | 43-115 | `assert_key_contracts` harness with current `Err(_) => assert!(false)` arms at lines 61, 69, 77, 86 — repair targets line 69 |
| `crates/vb_storage/src/kani_typed_partitioned_ids.rs` | 1-2 | `#![cfg(kani)]` and `#![forbid(unsafe_code)]` confirmed |
| `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | 1305-1450 | `proptest!` block with six over-rejecting specimens at lines 1326-1351, 1355-1369, 1373-1386, 1390-1401, 1405-1423, 1427-1449 |
| `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` | 123-146 | Canonical-positive reference proptest `run_event_ordering` with `s in 0u64..u64::MAX` (lines 129, 131) |

## User-Specified Verification

| Check | Result | Evidence |
|-------|--------|----------|
| 4 obligations (cargo-test + kani + source-lint) | **PASS** | `proof-obligations.planned.jsonl` has 4 rows: `PO-CARGO-TEST-001`, `PO-CARGO-LIB-001` (both `cargo-test`), `PO-KANI-001` (`kani`), `PO-LINT-SRC-001` (`source-lint`). `PO-MOON-CI-001` is deferred to state 12 and is not an active State-5 obligation. |
| Tighten seq range to `0u64..u64::MAX` at `restate_journal_tail_scan_fallback_tests.rs:1326-1449` | **PASS** | All 6 over-rejecting proptests are inside the range and are planned for the `0u64..u64::MAX` shrink: lines 1326-1351 (`run_event_key_lexicographic_ordering`, s1+s2), 1355-1369 (`sequence_bytes_roundtrip_through_key_encoding`, seq_val), 1373-1386 (`run_event_key_always_17_bytes`, seq_val), 1390-1401 (`run_event_key_always_has_correct_prefix`, seq_val), 1405-1423 (`different_runs_have_different_event_key_prefixes`, s1+s2), 1427-1449 (`same_run_different_seq_keys_differ_in_seq_bytes`, s1+s2). Repair shape matches `fjall_keyspace_manifest_tests.rs:129,131` (`s in 0u64..u64::MAX`). |
| Kani harness explicit match for `Err(SequenceOverflow)` | **PASS** | `proof-strategy.md` §5.2 + §9 (State 5) + §8 forbidden action #8 mandate the explicit match `Err(JournalError::SequenceOverflow) => assert!(seq_value == u64::MAX)`. `PO-KANI-001.repair_form` states the same shape; `Err(_) => assert!(false)` is retained only for the defensive non-`SequenceOverflow` arm. `proof-seeds.jsonl` row `ps-vb-uwxct-007` notes preferred repair option is the explicit match. |
| No blanket `kani::assume` | **PASS** | `proof-strategy.md` §8 forbidden action #8 forbids `kani::assume(seq_value != u64::MAX)`. `proof-strategy.md` §5.2 explicitly states no `kani::assume(seq_value != u64::MAX)` is added. `ps-vb-uwxct-007.notes` only mentions `kani::assume` as an acceptable-but-not-preferred alternative, and `PO-KANI-001.repair_form` does NOT include it. `trusted-base-plan.md` "Forbidden Categories" reiterates the ban. |

## Review Summary

### Lane Decision Coverage: PASS

- 64 lane decisions (VLD-001 through VLD-064) covering all 8 proof seeds (ps-vb-uwxct-000..007) × the 8 verifiers in the default Rust profile plus conditional lanes.
- Required lanes:
  - `cargo-test`: 8 (one per seed, anchor + C1..C6 + Kani-harness N/A via cfg-kani)
  - `source-lint`: 8 (one per seed; cross-cutting)
  - `kani`: 1 (ps-vb-uwxct-007, C7)
- Not-applicable lanes: 47 (verus×8, flux-rs×8, loom×8, miri×8, cargo-fuzz×8; plus kani×7 for proptest/anchor seeds where the harness is `#[cfg(kani)]-stripped`).
- Every proof seed has a full lane decision set for all 8 verifiers. No silent omissions.

### Non-Applicability Evidence: PASS

- **Verus (8 seeds)**: No production Rust change in scope; the production encoder at `crates/vb_storage/src/keys.rs:480-496` is reference-only. Adding a Verus proof obligation would be VACUUM (GOD RULE 2 — no vacuum Verus proofs). Each row carries `limitation_kind: no_production_target` and cites `crates/vb_storage/src/keys.rs:480-496 action=reference_only in delivery-scope.jsonl` plus the `proof-planner` Production Binding Plan STRONG/WEAK_MIRROR/WEAK_EXTERN requirement.
- **Flux-rs (8 seeds)**: No `#[refined_by]` / `#[spec]` annotations on the production encoder; no production refinement target exists.
- **Loom (8 seeds)**: No concurrent primitives in the touched test files. The proptest! block uses the default single-threaded deterministic runner; the Kani harness is a `#[cfg(kani)]` symbolic single-threaded model.
- **Miri (8 seeds)**: The Kani harness file carries `#![forbid(unsafe_code)]` (line 2); `restate_journal_tail_scan_fallback_tests.rs` is safe Rust only; no FFI/raw-pointer/MaybeUninit operations introduced.
- **cargo-fuzz (8 seeds)**: The repair is a data-domain shrink and a typed-error match — no new parser/codec surface. Proptest inputs are typed `RunId::new(r)` / `EventSeq::new(s)` wrappers, not raw bytes.
- **kani on proptest seeds (6)**: Proptest! blocks are not `#[kani::proof]` functions; the harness lives in `kani_typed_partitioned_ids.rs`.
- **cargo-test on kani harness (VLD-057)**: The Kani harness is `#[cfg(kani)]`-stripped under cargo test, so cargo test skips it; the harness is exercised by `cargo kani` instead.

Every `not_applicable` row cites concrete file:line evidence. No "not needed" / "too hard" rationales.

### Obligation Schema: PASS (with one observation)

- 4 obligations in `proof-obligations.planned.jsonl` use the conventional abbreviated `proof-obligation/v1` shape used elsewhere in the repo (e.g. vb-b8i8f, vb-2bzz, vb-0253.*). No legacy alias fields (`layer`, `checker`, `claim`) detected. `target` is implicit (cargo-test is target-implicit; kani is target-bound to `vb_eepg_typed_partitioned_ids` via `kani_module`+`harness`+`helper` fields; source-lint is target-implicit).
- Each obligation includes `command`, `expected_evidence`, `assumptions`, `mode`, `owner_state: 5`, `rerun_from: 4`, `status: planned`. Commands are concrete and runnable with explicit flags and workdir anchors.
- `PO-KANI-001` includes the optional but useful `production_binding` field (STRONG mechanism, `crates/vb_storage/src/keys.rs:81-83`, `assume_specification_targets: [production::run_event_key, production::sequenced_run_key]`). This is not mandatory because `verifier: kani` (not Verus), but it is exemplary discipline.

### TLA+ Compliance: PASS

- TLA+ globally removed per mandate. No TLA+ obligations, lane decisions, or waived lanes.

### Waiver Candidates: PASS

- 6 waiver rows: `WC-MASTER` (covers all clauses) + 5 lane-specific (`WC-001` Verus, `WC-002` Flux-rs, `WC-003` Loom, `WC-004` Miri, `WC-005` cargo-fuzz).
- All rows carry `behavior_affecting: false`. None waive behavior.
- Every row references the canonical-positive unit test anchor (`crates/vb_storage/src/keys/tests.rs:497-505`) and/or the existing Verus spec mirror as compensating evidence.
- `WC-MASTER.review_status: pending` — appropriate at planning state; promotion to `approved` belongs to State 6 (formal-verifier).

### Trusted Base Plan: PASS

- 4 entries (TBR-001..TBR-004): 2 `assume` (variant identity, integer packing), 2 `external_body` (canonical-positive reference pattern, production binding).
- Zero `unsafe`, `axiom`, `admit`, `axiom`, `cover!`-as-proof, or blanket `kani::assume` in executable proof code.
- `trusted-base-plan.md` "Forbidden Categories" explicitly enumerates each banned category and links to AGENTS.md "Formal Verification Mandates" + `proof-planner` skill "Anti-laundering".
- Every entry has compensating evidence anchored to either `PO-CARGO-LIB-001` (canonical-positive) or `PO-KANI-001` (Kani raw PASS).

### Production Binding Audit (GOD RULE 2): PASS

- The proof-plan-reviewer skill mandates `production_binding` on every Verus obligation row (STRONG / WEAK_MIRROR / WEAK_EXTERN). This bead creates **zero Verus obligations** because no production Rust code changes — adding one would be VACUUM and is forbidden by GOD RULE 2.
- The single Kani obligation `PO-KANI-001` carries a `production_binding: { mechanism: STRONG, production_path: crates/vb_storage/src/keys.rs, production_lines: 81-83 }` field documenting direct harness binding to production `run_event_key` (no mirror or shadow type).
- The Kani harness at `crates/vb_storage/src/kani_typed_partitioned_ids.rs:63-70` was inspected and confirmed to call `keys::run_event_key(run, seq)` directly (line 63), satisfying the STRONG binding requirement.
- No `EXPLICITLY_ALLOWED`, `ALLOWED_EXCEPTIONS`, or offload mechanism is used; the backdoor escape hatch is not employed.

### Non-Vacuity: PASS

- Proptest `cargo-test` obligations use the canonical `0u64..u64::MAX` shrink that exercises the encodable range; no `prop_assume!`-only path; the property under test must hold on every sampled input.
- `PO-KANI-001` uses `kani::Arbitrary`-derived `SymbolicKeyInputs` (lines 15-24) producing arbitrary `u16`/`u8` pairs; the packing formula `(hi<<16) | lo` reconstructs the u64 value (lines 35-41); the harness calls production `keys::run_event_key` directly. No `cover!`-only claim is used to satisfy the obligation.
- The explicit `Err(JournalError::SequenceOverflow) => assert!(seq_value == u64::MAX)` arm asserts a concrete equality, not a tautology.

### Bridge Planning: PASS

- `trusted-base-plan.md` Production Binding Audit + §8 forbidden actions in `proof-strategy.md` enumerate the production references and the forbidden actions for the proof-writer / proof-to-implementation downstream agents.
- No `proof-to-implementation-input.md` is required because there is no production Rust change; the bead is test-only. The repair does not require a bridge between proof claims and production Rust edits — the production contract is already verified by `keys/tests.rs:497-505` (canonical-positive) and the existing Verus spec mirror `verification/verus/extern_vb_storage_keys.rs` (out of scope per contract §2).

### Review Provenance: PASS

- Planner invocation: `vb-uwxct-state4-proof-planner-attempt1`
- Reviewer invocation: `vb-uwxct-state4-proof-plan-review-attempt1`
- The two invocation IDs differ; the planner artifact set does not self-stamp reviewer disposition fields (no `reviewer_disposition` key in any `proof-obligations.planned.jsonl` row, no `accepted` markers from the planner side).
- Independent review artifacts (`verifier-lane-review.jsonl`) written by this reviewer carry independent invocation IDs.

### `verifier-lane-review.jsonl`: PASS

- 64 review rows (VLR-001 through VLR-064), one per planner lane decision VLD-001..VLD-064.
- All 64 rows use `schema_version: verifier-lane-review/v1` with all required fields populated.
- All 64 rows carry `reviewer_disposition: accepted` (the plan is sound for every lane).
- All 64 rows cite the same planner invocation id and this reviewer's invocation id.
- `owner_state: 4`, `status: reviewed`.

## Findings

| ID | Code | Severity | Description |
|----|------|----------|-------------|
| (none) | — | — | No blocker / high / medium / low findings. The plan is precise, sound, and ready for proof-writer (State 5). |

The plan passes all rubric checks; no repair guide is required.

## Forbidden Actions Verified Against Plan

| Forbidden action | Plan status |
|------------------|-------------|
| Modify `crates/vb_storage/src/keys.rs:480-496` | NOT MODIFIED (reference-only per delivery-scope.jsonl); trust boundary confirmed |
| Modify `JournalError` enum or variants | NOT MODIFIED (production untouched) |
| Modify `verification/verus/extern_vb_storage_keys.rs` | NOT MODIFIED (out of scope per contract §2) |
| Touch `crates/vb_storage/src/keys/tests.rs:469-526` | NOT MODIFIED (canonical-positive reference) |
| Touch `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs:123-146` | NOT MODIFIED (canonical-positive reference) |
| Touch `crates/vb_runtime/src/journal/tests/chunk_004.rs:964-973` | NOT MODIFIED (event validity — separate invariant) |
| Touch `crates/vb_storage/src/proptests.rs` or `crates/vb_storage/src/kani_record_kind.rs` | NOT MODIFIED |
| Add blanket `kani::assume(seq_value != u64::MAX)` | NOT PLANNED (explicit match is the chosen repair) |
| Add new dependencies | NOT PLANNED |
| Use `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, `assert!(false)`, `[T]::last()`, unchecked indexing | NOT PLANNED in the six repaired specimens (`.expect(...)` is REMOVED by the range shrink; `Err(_) => assert!(false)` retained ONLY in the Kani harness defensive non-`SequenceOverflow` arm) |

## Verdict

The proof plan is precise, implementation-bound, and consistent with the canonical-positive reference pattern. All 64 lane decisions are justified with concrete file:line evidence. All 4 obligations have explicit commands, bounds, assumptions, and expected evidence. The trusted base is planned with compensating evidence. Zero behavior-affecting waivers exist. Production `keys.rs:480-496` is reference-only and stays untouched; the Kani harness is implementation-bound (STRONG) to production `run_event_key` at `keys.rs:81-83`. The plan is ready for proof-writer (State 5).

STATUS: APPROVED

## Next Steps

1. State 5 (proof-writer): Execute the 4 planned obligations using exact commands.
   - Tighten each proptest range from `s: u64` to `s in 0u64..u64::MAX` (six specimens).
   - Replace `Err(_) => assert!(false)` at `crates/vb_storage/src/kani_typed_partitioned_ids.rs:69` with explicit match `Err(JournalError::SequenceOverflow) => assert!(seq_value == u64::MAX)` plus retained `Err(_) => assert!(false)` defensive arm.
2. State 6 (proof-reviewer): Validate written proof artifacts (raw Kani PASS, raw cargo test PASS, raw source-lint PASS) against this plan.
3. State 8 (formal-verifier): Execute the 4 obligations and capture raw evidence under `.beads/vb-uwxct/evidence/`.
4. State 12 (Gauntlet): `PO-MOON-CI-001` is deferred for closure.