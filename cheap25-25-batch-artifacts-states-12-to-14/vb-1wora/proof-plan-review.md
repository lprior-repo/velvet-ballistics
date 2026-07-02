# Proof Plan Review — vb-1wora

## Reviewer Metadata

| Field | Value |
|-------|-------|
| reviewer_skill | proof-plan-reviewer |
| reviewer_invocation_id | ppr-vb-1wora-2026-07-01 |
| review_state | state_4_review |
| planner_invocation_id | planner-vb-1wora-state4-fwdport (fwd-port from `cheap25-vb-1wora` workspace; not independently recorded in `agent-invocation-ledger.jsonl`) |
| reviewed_artifacts | proof-strategy.md, verifier-lane-decisions.jsonl, proof-obligations.planned.jsonl, trusted-base-plan.md, waiver-candidates.jsonl |
| lanes_reviewed | 10 (1 bead × 10 verifier lane rows = 6 required + 4 not_applicable) |
| planner_obligations | 7 (POB-vb-1wora-001..007) |

## Reviewed Artifacts (SHA-256)

| Artifact | SHA-256 |
|----------|---------|
| `proof-strategy.md` | `9cde034d846b54d18c2a8952b95b03d1a0af06f06545f35a831501b1b8f6c05a` |
| `verifier-lane-decisions.jsonl` | `f09572121fa16a8f04d798c999a77dfdba12e33fc6d1d5cd020eec80ec64b54f` |
| `proof-obligations.planned.jsonl` | `6ac3d767f5803c73e0dda9fcc5c0d094407676e19aae14e158170cbd2d36b6e4` |
| `trusted-base-plan.md` | `c7ddd13e44a243b32fee3ca524cb110c79d1621079b7865f0a0e2116b6265398` |
| `waiver-candidates.jsonl` | `4766bc9aa3d66fea50fe09bbe2654b1fa3d40e1d95170036ddfe62ab27331a81` |

## Schema Integrity

| Artifact | Schema | Result |
|----------|--------|--------|
| `verifier-lane-decisions.jsonl` | `verifier-lane-decision/v1` | PASS — 10 rows, all `schema_version` present, all required fields populated |
| `proof-obligations.planned.jsonl` | `proof-obligation/v1` | PASS — 7 rows, all `schema_version` present, all `command`/`workdir`/`expected_evidence`/`model_bounds`/`assumptions`/`trusted_base_refs` populated |
| `waiver-candidates.jsonl` | `waiver-candidate/v1` | PASS — 5 rows, all `behavior_affecting: false`, all `boundary_proof` concrete |

## God-Rule Compliance

| Check | Result | Evidence |
|-------|--------|----------|
| No behavior-affecting waivers | PASS | All 5 waiver-candidate rows are `behavior_affecting: false` (non-applicability statements for loom/miri/flux/tla-plus + recommended-only CODE_REGISTRATION, not waivers in the E_BEHAVIOR_WAIVER sense) |
| Verus production_binding present | PASS | POB-vb-1wora-006 declares `production_binding.mechanism: WEAK_MIRROR` with all 5 required fields populated and verified on disk |
| No `EXPLICITLY_ALLOWED` / `ALLOWED_EXCEPTIONS` / `OFFLOAD` escape hatches | PASS | Production-binding mechanism is `WEAK_MIRROR` only |
| Vacuous-proof prohibition (GOD RULE 2) | PASS | Verus spec binds to production via `extern_vb_vzcuf_PS_003.rs:71-72` (`#[path = "production_inner/vb_vzcuf_PS_003_production.rs"]`) which reaches the production source; exec wrappers `wrapper_decode_record_ok` (line 808), `wrapper_decode_record_bad_magic` (line 887), `wrapper_decode_record_parity_mismatch` (line 968) exercise the bridge; drift gate `scripts/check-production-inner-drift.sh` enforces mirror parity; production-binding gate `scripts/check-verus-production-binding.sh` enforces bridge arm parity |
| Kani hardcoded shapes prohibited (GOD RULE 1) | PASS | POB-vb-1wora-004 uses `kani::any()` for `valid_magic: u32`, `payload_len: u32`, and `payload: Vec<u8>` (symbolic); only the trailing byte count is concrete (1..=8) per A-001, and the harness asserts `result` shape rather than trusting any hardcoded fixture |
| TLA+ bounded arithmetic (GOD RULE 3) | N/A | TLA+ lane is `not_applicable`; no TLA+ obligations exist |
| No loop oscillation (GOD RULE 4) | N/A | Plan-only artifact; no implementation yet |
| Differential verification only (GOD RULE 5) | PASS | Trimmed scope: 7 obligations cover exactly the 7 INV-CODEC-TB-* invariants + HOSTILE-INPUT-001; no fleet-wide blind mutation |

## Lane Disposition Summary

| Lane | Required | Not Applicable | Accepted | Rejected |
|------|---------:|---------------:|---------:|---------:|
| rust-local | 1 | 0 | 1 | 0 |
| cargo-test | 1 | 0 | 1 | 0 |
| proptest | 1 | 0 | 1 | 0 |
| kani | 1 | 0 | 1 | 0 |
| verus | 1 | 0 | 1 | 0 |
| cargo-fuzz | 1 | 0 | 1 | 0 |
| loom | 0 | 1 | 1 | 0 |
| miri | 0 | 1 | 1 | 0 |
| flux | 0 | 1 | 1 | 0 |
| tla-plus | 0 | 1 | 1 | 0 |
| **Total** | **6** | **4** | **10** | **0** |

## Obligation Coverage

| POB | Lane | Closes Invariants |
|-----|------|-------------------|
| POB-vb-1wora-001 | rust-local (structural review) | INV-CODEC-TB-003, INV-CODEC-TB-006 |
| POB-vb-1wora-002 | cargo-test (variant trio + test inversion + mirror test) | INV-CODEC-TB-001, INV-CODEC-TB-005, INV-CODEC-TB-006, mirror INV-CODEC-TB-004 |
| POB-vb-1wora-003 | proptest (round-trip + mutual exclusion) | INV-CODEC-TB-002, INV-CODEC-TB-010, REFINE-MUTEX-001 |
| POB-vb-1wora-004 | kani (H6: trailing-before-digest) | INV-CODEC-TB-001, INV-CODEC-TB-003, INV-CODEC-TB-005 |
| POB-vb-1wora-005 | proptest (random byte-append oracle) | INV-CODEC-TB-002, INV-CODEC-TB-004, INV-CODEC-TB-009 |
| POB-vb-1wora-006 | verus (PS-003 bridge, WEAK_MIRROR) | INV-CODEC-TB-007, partial INV-CODEC-TB-001 |
| POB-vb-1wora-007 | cargo-fuzz (hostile-input oracle) | HOSTILE-INPUT-001, partial INV-CODEC-TB-005 |

All 7 contract invariants (INV-CODEC-TB-001..007) and HOSTILE-INPUT-001 are covered; the 7 obligations stay within the 5–7 bead budget.

## Findings

No blocker findings. No minor / observation / informational findings. Plan is precise enough for proof-writer and proof-to-implementation.

## Verifier-Lane-Review Disposition

All 10 `verifier-lane-review/v1` rows written to `verifier-lane-review.jsonl`:
- 6 required lanes (rust-local, cargo-test, proptest, kani, verus, cargo-fuzz): all `accepted` with explicit command evidence and (for Verus) production_binding validation
- 4 not_applicable lanes (loom, miri, flux, tla-plus): all `accepted` with concrete `non_applicability_evidence_refs` and `limitation_kind`

Each row carries an independent `reviewer_invocation_id` (`ppr-vb-1wora-2026-07-01`) distinct from the planner invocation ID, satisfying the reviewer-provenance check.

## Repair-Guide Status

`proof-plan-repair-guide.md` is NOT generated (no rejected lane, no blocker finding, no required repair).

## Positive Findings

1. **Production-binding gate compliance (GOD RULE 2).** POB-vb-1wora-006 declares `WEAK_MIRROR` with all five required fields populated and validated: `production_path=crates/vb_storage/src/codec/payload.rs` exists; `production_lines=56-82` matches the mirror's documented source range at `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:72` ("`decode_record_payload -> codec/payload.rs:56-82 -> spec_decode_record_payload`"); `mirror_path` exists; `drift_gate_script=scripts/check-production-inner-drift.sh` exists; `drift_threshold=zero`. The layered WEAK_EXTERN shim (`extern_vb_vzcuf_PS_003.rs:71-72` does `#[path = "production_inner/vb_vzcuf_PS_003_production.rs"]`) further strengthens the binding. Direct STRONG `#[path]` binding is correctly rejected because production reaches `postcard`, `blake3`, and `crc32c` which are not Verus-modelable in single-file `verus --crate-type=lib`.
2. **TrailingBytes variant placement.** `crates/vb_storage/src/error/mod.rs:96-97` shows `UnexpectedEof` then `MalformedKeyspaceRow`. The contract and proof-strategy both place `TrailingBytes { trailing: usize }` between `UnexpectedEof` (line 96) and `MalformedKeyspaceRow` (line 98), which mirrors the bridge's intended ordering (slice → trailing check) and is consistent with `PS-VB-1WORA-009` (REFINE-MUTEX-001).
3. **0x4042 collision check.** `crates/vb_storage/src/error/codes.rs` line 81 documents `REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE: DiagnosticCode::new(0x4041)` as the highest used `0x40xx` slot for journal errors. `0x4042` is verified free in `codebase-map.md:112-123` and `codebase-map.md:148-199` (numeric-code recommendations section). `POB-vb-1wora-002`'s `trailing_bytes_error_has_correct_code` test locks the `0x4042` constant numerically and the wiring parity.
4. **Pre-fix test inversion is concrete.** `crates/vb_storage/src/codec/tests.rs:1499` is the exact location of `decode_ignores_trailing_bytes_beyond_payload`, which uses the `0xFF 0xFE 0xFD` 3-byte fixture asserted via `assert_eq!(decoded, event, "trailing bytes should be ignored on decode");`. POB-vb-1wora-002 specifies the rename to `decode_rejects_trailing_bytes_after_payload` with assertion `matches!(result, Err(JournalError::TrailingBytes { trailing: 3 }))` — a direct test inversion, not a parallel test.
5. **Kani H6 inherits H5's proven shape.** `crates/vb_storage/src/kani_postcard_envelope_wire.rs:271-337` is H5 (`kani_harness_digest_before_postcard`) using `#[kani::unwind(4)]` for the same 60-byte header + arbitrary `payload_len: u32` bounded by `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`. POB-vb-1wora-004 specifies H6 inserted after H5, inheriting `#[kani::unwind(4)]`, stubbing `verify_digest_match` as a counted call (Kani's recommended stubbing pattern from `crates/vb_storage/src/kani_record_magic.rs:16-127`).
6. **Mirror test covers INV-CODEC-TB-004.** POB-vb-1wora-002's `decode_envelope_only_rejects_trailing_payload` (4-byte fixture, `Err(TrailingBytes { trailing: 4 })`) and POB-vb-1wora-005's `proptest_decode_envelope_only_rejects_random_trailing` lock the mirror site at `crates/vb_storage/src/codec/envelope.rs:48-83` in lockstep with `decode_record_payload`.
7. **Round-trip preservation is mandatory.** POB-vb-1wora-003 explicitly covers ROUND-TRIP-001 (existing encode → decode round-trip must continue to pass) and POB-vb-1wora-007 (cargo-fuzz) feeds randomly-appended bytes to verify the encoder never produces trailing bytes. The contract's §4.3 documents the encoder as out of scope.
8. **Trusted-base scoping is precise.** TBP-VB-1WORA-006 (mirror drift gate) and TBP-VB-1WORA-007 (Verus toolchain + production-binding gate) are the only "external" trusted bases; everything else is the Rust type system or the existing test suite.
9. **Fuzz fallback is documented.** A-005 explicitly handles the case where `cargo +nightly fuzz` is unavailable (degrade to `blocked_tooling`, POB-vb-1wora-002 directed tests still cover the invariant). No silent omission.
10. **Forbidden-pattern table is enforced.** `proof-strategy.md §7` enumerates 7 forbidden patterns (no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`, no `TrailingBytes { trailing: 0 }`, no hand-written shadow types without `#[path]`, no encoder modification, etc.) with the corresponding obligation that catches each one.

## Cross-References

- `proof-strategy.md` (24.4K) — full strategy document with §1 scope, §2 verifier selection, §3 risk classification, §4 per-obligation detail, §5 coverage summary, §6 assumptions, §7 forbidden patterns, §8 risk register, §9 handoff
- `verifier-lane-decisions.jsonl` (10.8K) — 10 lane decisions (6 required + 4 not_applicable) all with concrete `non_applicability_evidence_refs`
- `proof-obligations.planned.jsonl` (22.8K) — 7 obligations with explicit `command`, `workdir`, `expected_evidence`, `model_bounds`, `assumptions`, `trusted_base_refs`; POB-vb-1wora-006 carries `production_binding`
- `trusted-base-plan.md` (14.1K) — 9 trusted surfaces with concrete justifications
- `waiver-candidates.jsonl` (8.1K) — 5 non-applicability candidates (loom, miri, flux, tla-plus, code-registry), all `behavior_affecting: false`
- `contracts/proof-seeds.jsonl` (10 rows) — PS-VB-1WORA-001..010
- `contracts/traceability-matrix.jsonl` (10 rows) — TM-VB-1WORA rows mapping each invariant to source/test/verifier

## Handoff

State 5 (`proof-writer`) may proceed. The proof-writer is responsible for:

1. Adding `JournalError::TrailingBytes { trailing: usize }` to `crates/vb_storage/src/error/mod.rs` (between line 96 `UnexpectedEof` and line 98 `MalformedKeyspaceRow`).
2. Adding `TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042)` to `crates/vb_storage/src/error/codes.rs` (next slot after line 81's `0x4041`).
3. Adding the diagnostic_code() and symbolic_code() match arms for `TrailingBytes` in `crates/vb_storage/src/error/codes.rs`.
4. Inserting the trailing-bytes check in `crates/vb_storage/src/codec/payload.rs:56-82` (between `bytes.get(...).ok_or(UnexpectedEof)?;` at line 69-71 and `verify_digest_match(...)` at line 72) and the symmetric check in `crates/vb_storage/src/codec/envelope.rs:48-83`.
5. Renaming `decode_ignores_trailing_bytes_beyond_payload` → `decode_rejects_trailing_bytes_after_payload` in `crates/vb_storage/src/codec/tests.rs:1498-1524` and adding `decode_envelope_only_rejects_trailing_payload` + the variant trio + `trailing_bytes_error_has_correct_code`.
6. Adding `SpecJournalError::TrailingBytes { trailing: u32 }` to `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413` (between `UnexpectedEof` at line 388 and `PostcardDecodeFailed` at line 392) and updating the enumeration comment at lines 280-327.
7. Adding the bridge `Err(SpecJournalError::TrailingBytes { trailing }) => { ... }` arm to `verification/verus/vb-vzcuf-PS-003.rs:387-451` (`assume_specification[ production::decode_record ]`) with the precondition `(bytes.len() as u32) > expected_payload_end && trailing == (bytes.len() as u32) - expected_payload_end && trailing > 0`. The proof-writer must add a corresponding exec wrapper that exercises this arm.
8. Adding `kani_harness_rejects_trailing_bytes` (H6) to `crates/vb_storage/src/kani_postcard_envelope_wire.rs` after H5 at line 337.
9. Adding `proptest_trailing_bytes_*` properties to `crates/vb_storage/src/codec/tests.rs` under a `#[cfg(test)] mod proptests` block.
10. Adding `fuzz_target_trailing_bytes` to `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs`.

The proof-writer MUST NOT touch production Rust beyond items 1-4 above (which are the agreed delivery scope per `contracts/contract.md §4.1`). Items 5-10 are test/proof artifacts only.

State 7 (`proof-to-implementation`) will then bridge each claim to its Rust source ref and behavior-test fixture.

State 12 (`formal-verifier`) will execute each obligation's `command` and close the ledger.

---

## STATUS: APPROVED