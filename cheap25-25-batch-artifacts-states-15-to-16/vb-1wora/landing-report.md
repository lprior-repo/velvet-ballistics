# Landing Report — vb-1wora

## Bead: vb-1wora — Codec: reject trailing bytes after declared record payload (P1)
## State: 15 (Landing)
## Workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
## Date: 2026-07-02

---

## Landing Summary

| Field | Value |
|-------|-------|
| bead_id | vb-1wora |
| landing_type | Bug fix — production code change in vb_storage |
| source_checkout | /home/lewis/src/velvet-ballistics |
| isolated_workspace | /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora |
| jj_workspace | cheap25-vb-1wora (already cleaned up post-landing) |
| jj_working_commit | vlyqryto ba210bf8 (p11-holzman-rust — reject trailing bytes in codec) |
| git_remote | origin/main |
| dolt_remote | https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics |

---

## Evidence of Prior State Approvals

| State | Artifact | Status |
|-------|----------|--------|
| S1 (Go-skill) | STATE.md, baseline-report.md | COMPLETED |
| S2 (Explore) | codebase-map.md, delivery-scope.jsonl | COMPLETED |
| S3 (Rust-contract) | contracts/*.cue, contracts/*.md | COMPLETED |
| S4 (Proof-plan-review) | proof-plan-review.md, verifier-lane-review.jsonl | APPROVED |
| S5 (Proof-writer) | proof-writer-report.md, verification/verus/vb-vzcuf-PS-003.rs, fuzz_targets/*, kani_postcard_envelope_wire.rs | COMPLETED |
| S11 (Holzman-rust) | implementation.md, evidence/cargo_*.log | IMPLEMENTED |
| S12 (Formal-verifier) | formal-verification-report.md, verification-ledger.jsonl | APPROVED_WITH_BLOCKED_TOOLING |
| S13 (Black-hat-reviewer) | black-hat-review.md | APPROVED |
| S14 (Evidence-packaging + truth-serum) | assurance-bundle.md, truth-serum-report.md, final-evidence-decision.md | APPROVED |

---

## Deliverables (committed to isolated workspace)

**Production additions (crates/vb_storage):**

- `crates/vb_storage/src/error/mod.rs`: new `JournalError::TrailingBytes { trailing: usize }` variant at line 99
- `crates/vb_storage/src/error/codes.rs`: `TRAILING_BYTES_CODE = 0x4042` constant; diagnostic_code + symbolic_code arms wired (lines 85, 110, 165)
- `crates/vb_storage/src/codec/payload.rs`: trailing-bytes check inserted at lines 76-83 (before `verify_digest_match`)
- `crates/vb_storage/src/codec/envelope.rs`: same check at lines 77-84 (mirror site)

**Test additions (crates/vb_storage):**

- `crates/vb_storage/src/codec/payload.rs:194-205`: `decode_rejects_trailing_bytes_after_payload` (inverted from silent-accept)
- `crates/vb_storage/src/codec/envelope.rs:194-205`: `decode_envelope_only_rejects_trailing_payload` (mirror test)
- `crates/vb_storage/src/error/error_tests.rs`: trio `trailing_bytes_variant_and_fields`, `trailing_bytes_display_format`, `trailing_bytes_error_code`
- `crates/vb_storage/src/error/error_code_tests.rs:144-160`: `trailing_bytes_error_has_correct_code`
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs`: `ps003_trailing_bytes_are_rejected` (1024 cases, trailing_len in 1..=8) and `ps003_exact_boundary_roundtrips` (1024 cases)
- `crates/vb_storage/tests/security_tests.rs`: `zero_payload_len_with_bytes_fails_digest_check` updated to assert TrailingBytes under the new ordering

---

## Targeted Verification Results

| Gate | Result | Raw evidence |
|------|--------|--------------|
| cargo check -p vb_storage --all-features | PASS | evidence/cargo_test_vb_storage.log |
| cargo check -p vb_storage --all-features --tests | PASS | evidence/cargo_test_vb_storage.log |
| cargo test -p vb_storage --all-features | 1678 passed (17 suites) | evidence/po-cargo-test-all-features.log |
| cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 | 8 passed | evidence/cargo_test_proptest_vb_vzcuf_PS_003.log |
| cargo clippy (strict, source-target) | No issues | evidence/cargo_clippy_vb_storage.log |
| cargo fmt --check -p vb_storage | clean | evidence/cargo_fmt_vb_storage.log |
| cargo test trailing-bytes direct (6 tests) | 6 passed | evidence/po-002-cargo-test-trailing-bytes-direct.log |
| cargo fuzz fuzz_storage_codec_payload_corruption 60s | 37080025 runs, 0 crashes | evidence/po-007-fuzz-trailing-bytes-60s.log |
| Verus bridge | 25 verified, 0 errors | evidence/po-006-verus-ps-003-bridge-trailing-bytes.log |

---

## Formal Verification Status

7 Proof Obligations closed at State 12:

| POB | Verifier | Result |
|-----|----------|--------|
| POB-vb-1wora-001 (decoder ordering, diagnostic wiring) | rust-local | PASS |
| POB-vb-1wora-002 (trailing-bytes rejection behavior) | cargo-test | PASS |
| POB-vb-1wora-003 (round-trip preservation proptest) | proptest | PASS |
| POB-vb-1wora-004 (Kani H6 syntax + ordering) | kani | BLOCKED_TOOLING (pre-existing kani_helpers.rs:22) |
| POB-vb-1wora-005 (mirror site proptest) | proptest | PASS |
| POB-vb-1wora-006 (Verus bridge + binding gate) | verus | PASS+BLOCKED_TOOLING (drift gate BLOCKED_TOOLING, JJ-only workspace) |
| POB-vb-1wora-007 (hostile-input fuzz oracle) | cargo-fuzz | PASS |

5 formal-waivers (Loom, Miri, Flux, TLA+, CODE_REGISTRY registration), all `behavior_affecting: false`, all `not_applicable`.

---

## Push Evidence

```
git push:        pending (cheap25-25 batch landing orchestrated by femdation dispatch)
bd dolt push:    COMPLETE — pushed to https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics
bead close:      COMPLETE — bd close vb-1wora
```

---

## Next Steps

1. `bd dolt push` — COMPLETED (this session)
2. `bd close vb-1wora --reason "..."` — COMPLETED (this session)
3. Cheap25-25 batch merge to main — orchestrated by femdation dispatch agent (outside this bead's scope)

---

## SIGNATURE

```
BEAD: vb-1wora
STATE: 15 (landing)
STATUS: LANDING_COMPLETE
NEXT_GATE: cleanup (state 16) + bead closure confirmation
```