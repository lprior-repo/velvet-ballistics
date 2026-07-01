# Proof Coverage Matrix: vb-uwxct

## Requirement-to-Obligation Traceability

| Requirement | Contract Clause | Proof Seed | cargo-test | kani | source-lint | Verus | Flux-rs | Loom | Miri | cargo-fuzz |
|-------------|-----------------|------------|------------|------|-------------|-------|---------|------|------|------------|
| REQ-vb-uwxct-encoder (anchor) | C0 | ps-vb-uwxct-000 | PO-CARGO-LIB-001 | — | PO-LINT-SRC-001 | — | — | — | — | — |
| REQ-vb-uwxct-proptest-lex-ordering | C1 | ps-vb-uwxct-001 | PO-CARGO-TEST-001 | — | PO-LINT-SRC-001 | — | — | — | — | — |
| REQ-vb-uwxct-proptest-seq-roundtrip | C2 | ps-vb-uwxct-002 | PO-CARGO-TEST-001 | — | PO-LINT-SRC-001 | — | — | — | — | — |
| REQ-vb-uwxct-proptest-always-17-bytes | C3 | ps-vb-uwxct-003 | PO-CARGO-TEST-001 | — | PO-LINT-SRC-001 | — | — | — | — | — |
| REQ-vb-uwxct-proptest-always-correct-prefix | C4 | ps-vb-uwxct-004 | PO-CARGO-TEST-001 | — | PO-LINT-SRC-001 | — | — | — | — | — |
| REQ-vb-uwxct-proptest-different-runs-prefix | C5 | ps-vb-uwxct-005 | PO-CARGO-TEST-001 | — | PO-LINT-SRC-001 | — | — | — | — | — |
| REQ-vb-uwxct-proptest-same-run-diff-seq | C6 | ps-vb-uwxct-006 | PO-CARGO-TEST-001 | — | PO-LINT-SRC-001 | — | — | — | — | — |
| REQ-vb-uwxct-kani-harness | C7 | ps-vb-uwxct-007 | — | PO-KANI-001 | PO-LINT-SRC-001 | — | — | — | — | — |

## Coverage Legend

- **—**: Verifier is `not_applicable` for this seed (see `verifier-lane-decisions.jsonl` for evidence refs and `verifier-lane-matrix.md` for the matrix).
- **PO-XXXX-###**: Planned proof obligation ID; `planned` status, `owner_state: 4` (this state) or 5 (proof-writer) for test execution.
- The same obligation ID may cover multiple seeds; e.g. `PO-CARGO-TEST-001` covers C1..C6 by exercising one targeted test binary that contains all six tightened proptests.

## Required-Lane Coverage Summary

| Lane | Required obligations | Seeds covered |
|------|----------------------|----------------|
| cargo-test | 2 (PO-CARGO-TEST-001, PO-CARGO-LIB-001) | All 8 (C0 reference; C1..C6 tightened) |
| kani | 1 (PO-KANI-001) | C7 (typed-error match in harness) |
| source-lint | 1 (PO-LINT-SRC-001) | All 8 (cross-cutting) |

## Total Counts

- Proof seeds: **8** (ps-vb-uwxct-000..007)
- Required proof obligations: **4** (3 lane obligations + 1 deferred Gauntlet)
  - cargo-test: 2 (PO-CARGO-TEST-001, PO-CARGO-LIB-001)
  - kani: 1 (PO-KANI-001)
  - source-lint: 1 (PO-LINT-SRC-001)
  - deferred (State 12): 1 (PO-MOON-CI-001)
- Verus obligations: **0** (no production change → VACUUM avoided)
- Flux-rs obligations: **0**
- Loom obligations: **0** (no concurrency surface)
- Miri obligations: **0** (no unsafe surface)
- cargo-fuzz obligations: **0** (no new parser/codec surface)
- **Total planned obligations: 4** (3 + 1 deferred) — within the 3–4 obligation budget requested by the bead
- Behavior-affecting obligations: **0** (this is a test-only repair)

## Cross-Reference

- `verifier-lane-decisions.jsonl` — 64 rows (8 seeds × 8 verifiers) with `applicability` and `non_applicability_evidence_refs`.
- `proof-obligations.planned.jsonl` — 4 obligation rows with `evidence_command`, `expected_evidence`, `owner_state`, `rerun_from`, `status`.
- `trusted-base-plan.md` — 4 trusted-base entries (TBR-001..TBR-004), all reference/assume, no `unsafe`/`axiom`/`admit` in executable proof code.
- `waiver-candidates.jsonl` — 5 non-behavior-affecting waivers (WC-001..WC-005) plus 1 master "no behavior-affecting waivers" entry (WC-MASTER).

## Verification Gating Chain

```
State 4 (this state):
  proof-strategy.md            ✓
  verifier-lane-matrix.md      ✓
  verifier-lane-decisions.jsonl✓
  proof-coverage-matrix.md     ✓  ← THIS ARTIFACT
  proof-obligations.planned.jsonl
  trusted-base-plan.md
  waiver-candidates.jsonl

State 4b (proof-plan-reviewer): review 64 lane decisions + 4 obligations + 4 trusted-base entries + 5 waivers

State 5 (proof-writer — test edits):
  - 6 proptest range shrinks at restate_journal_tail_scan_fallback_tests.rs:1326-1449
  - Kani harness typed-error match at kani_typed_partitioned_ids.rs:63-70

State 8 (formal-verifier — execution):
  - PO-CARGO-TEST-001 (targeted test binary)
  - PO-CARGO-LIB-001 (vb_storage lib unit reference)
  - PO-KANI-001 (Kani harness probe)
  - PO-LINT-SRC-001 (forbidden-scan + source-length + clippy)

State 12 (Gauntlet):
  - PO-MOON-CI-001 (deferred)
```

## Lane Coverage Risks

- **Behavior is test-only**: the production encoder at `keys.rs:480-496` is
  reference-only and not bound to a Verus or Flux obligation. This is
  intentional and required by GOD RULE 2 (no VACUUM proofs). The
  contract is already verified by `keys/tests.rs:497-505` (canonical-positive)
  and the existing `verification/verus/extern_vb_storage_keys.rs` spec mirror.
- **The Kani harness is implementation-bound**: `kani_typed_partitioned_ids.rs:63-70`
  directly calls `keys::run_event_key` (the production symbol at
  `crates/vb_storage/src/keys.rs:81-83`); no shadow model is introduced. The
  repair uses an explicit match arm on `JournalError::SequenceOverflow` and
  asserts `seq_value == u64::MAX` on the typed rejection — preserving the
  production contract in the symbolic model.
- **No blanket `kani::assume` is added**: the contract clause C7 is satisfied
  by classifying the typed rejection, not by masking it. This preserves the
  sentinel in the proof model and prevents a regression if the encoder later
  starts over-rejecting or over-accepting.

## Obligation→Seed Mapping Detail

| Obligation | Seeds | Verifier | Evidence artifact |
|------------|-------|----------|--------------------|
| PO-CARGO-TEST-001 | C1, C2, C3, C4, C5, C6 (all six tightened proptests) | cargo-test | `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests -- --nocapture` exit 0 |
| PO-CARGO-LIB-001 | C0 (canonical-positive reference) | cargo-test | `cargo test -p vb_storage --lib keys::tests::` exit 0; `run_event_key_rejects_event_seq_max_sentinel` and `run_event_key_with_zero_seq` remain green |
| PO-KANI-001 | C7 (typed-error match) | kani | `bash scripts/kani-list.sh vb_storage` produces `kani-list.json` containing `vb_eepg_typed_partitioned_ids`; package-level Kani probe of the harness returns PASS; raw Kani log captured in `.beads/vb-uwxct/evidence/kani.log` |
| PO-LINT-SRC-001 | All seeds (cross-cutting) | source-lint | `bash scripts/forbidden-scan.sh` exits 0; `bash scripts/check-source-length.sh` exits 0; `cargo clippy --workspace --all-targets -- -D warnings` exits 0 |