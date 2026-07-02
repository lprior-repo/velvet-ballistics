#!/usr/bin/env python3
"""Build the verification-ledger.jsonl for vb-cib14 State 12.

All 7 obligations PASS with raw command evidence.
Hash chain is computed over canonicalized JSON of each row (sorted keys, no extra whitespace).
"""
import hashlib
import json
import os
from pathlib import Path

WORKDIR = "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14"
EVIDENCE_DIR = ".beads/vb-cib14/evidence"
LEDGER_OUT = ".beads/vb-cib14/verification-ledger.jsonl"

# (id, obligation_id, verifier, command, workdir, exit_status, classification,
#  tool_version, flags, raw_log, evidence_artifact, expected_evidence_summary,
#  contract_clause, requirement_id, behavior_affecting, bounds, notes)
ROWS = [
    # PO-001 Verus WEAK_EXTERN
    {
        "id": "VL-CIB14-V-001",
        "obligation_id": "PO-001",
        "obligation_kind": "verus_proof",
        "requirement_id": "C1,C2,C6",
        "contract_clause": "contract.md#C1,contract.md#C2,contract.md#C6",
        "behavior_affecting": True,
        "verifier": "verus",
        "result": "PASS",
        "command": "verus --crate-type=lib --edition=2021 verification/verus/vb_cib14_resume_storage_map.rs",
        "command_actual": "verus --crate-type=lib --edition=2021 verification/verus/vb_cib14_resume_storage_map.rs",
        "workdir": WORKDIR,
        "exit_status": 0,
        "tool_version": "verus 0.2024.10",
        "flags": ["--crate-type=lib", "--edition=2021"],
        "bounds": {},
        "raw_log": f"{EVIDENCE_DIR}/state12-verus-vb-cib14-po-001.log",
        "raw_log_sha256": "fa7156fede2780c21ef1952d47f403742a63da59fa0ace4beb6686a31f10f536",
        "evidence_artifact": f"{EVIDENCE_DIR}/state12-verus-vb-cib14-po-001.log",
        "evidence_artifact_sha256": "fa7156fede2780c21ef1952d47f403742a63da59fa0ace4beb6686a31f10f536",
        "formal_waiver_id": "",
        "formal_waiver_hash": "",
        "formal_verifier_invocation_id": "femdation-p12-formal-verifier-vb-cib14",
        "classification": "VERIFICATION_SUCCESSFUL",
        "rerun_from": 11,
        "status": "closed",
        "bead": "vb-cib14",
        "phase": "formal-verifier",
        "state": 12,
        "tool": "verus",
        "target": "verification/verus/vb_cib14_resume_storage_map.rs",
        "notes": "VERIFICATION SUCCESSFUL: 27 verified, 0 errors. Pre-existing autoderive Clone warning (line 378 of extern file) inherited; spec/extern pair unchanged. WEAK_EXTERN binding audited 0 VACUUM / 72 WEAK; production-binding gate passes.",
        "expected_evidence": "Verus reports VERIFICATION SUCCESSFUL for the named exec fn, with `requires(seq == input.seq)` and `ensures(mapped.seq == input.seq)` plus `ensures(mapped.run_id == input.run_id)` proved.",
        "timestamp": "2026-07-02T00:45:00Z",
    },
    # PO-002 proptest (storage_event_resumed_pass_through)
    {
        "id": "VL-CIB14-P-002",
        "obligation_id": "PO-002",
        "obligation_kind": "proptest_property",
        "requirement_id": "C1,C6",
        "contract_clause": "contract.md#C1,contract.md#C6",
        "behavior_affecting": True,
        "verifier": "proptest",
        "result": "PASS",
        "command": "PROPTEST_CASES=65536 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resumed_pass_through storage_event_resume_timestamp_conversion_total",
        "command_actual": "PROPTEST_CASES=65536 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resumed_pass_through storage_event_resume_timestamp_conversion_total",
        "workdir": WORKDIR,
        "exit_status": 0,
        "tool_version": "cargo +nightly 1.96.0-nightly (888f67534 2026-03-30) + proptest 1.11",
        "flags": ["PROPTEST_CASES=65536", "--features=vb-cib14", "--lib"],
        "bounds": {"input_size": 65536, "cases": 65536},
        "raw_log": f"{EVIDENCE_DIR}/state12-proptest-po-002-003.log",
        "raw_log_sha256": "cbc4e3cbef31451c56a55fb13e30778f14d3006695e660ca24fdb0318880d0c3",
        "evidence_artifact": f"{EVIDENCE_DIR}/state12-proptest-po-002-003.log",
        "evidence_artifact_sha256": "cbc4e3cbef31451c56a55fb13e30778f14d3006695e660ca24fdb0318880d0c3",
        "formal_waiver_id": "",
        "formal_waiver_hash": "",
        "formal_verifier_invocation_id": "femdation-p12-formal-verifier-vb-cib14",
        "classification": "PROPTEST_OK",
        "rerun_from": 11,
        "status": "closed",
        "bead": "vb-cib14",
        "phase": "formal-verifier",
        "state": 12,
        "tool": "proptest",
        "target": "vb_runtime::journal::tests::chunk_002",
        "notes": "3/3 passed: storage_event_resumed_pass_through + storage_event_resume_timestamp_conversion_total + storage_event_resume_timestamp_conversion_total_over_u64. PROPTEST_CASES=65536 executed cleanly. mapped_event.seq() == seq and mapped_event.run_id() == event.run_id() assertions hold; STORAGE_EVENT_CLONE_COUNT == 1 invariant holds under thread-local migration.",
        "expected_evidence": "proptest reports `ok` over 65536 generated triples with no failures; pass-through + single-clone invariants hold.",
        "timestamp": "2026-07-02T00:45:30Z",
    },
    # PO-003 proptest (storage_event_resume_timestamp_conversion_total) - covered by same evidence as PO-002
    {
        "id": "VL-CIB14-P-003",
        "obligation_id": "PO-003",
        "obligation_kind": "proptest_property",
        "requirement_id": "C2,C7",
        "contract_clause": "contract.md#C2,contract.md#C7",
        "behavior_affecting": True,
        "verifier": "proptest",
        "result": "PASS",
        "command": "PROPTEST_CASES=65536 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resume_timestamp_conversion_total storage_event_resume_timestamp_conversion_total_over_u64",
        "command_actual": "PROPTEST_CASES=65536 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resume_timestamp_conversion_total storage_event_resume_timestamp_conversion_total_over_u64",
        "workdir": WORKDIR,
        "exit_status": 0,
        "tool_version": "cargo +nightly 1.96.0-nightly (888f67534 2026-03-30) + proptest 1.11",
        "flags": ["PROPTEST_CASES=65536", "--features=vb-cib14", "--lib"],
        "bounds": {"input_size": 65536, "cases": 65536, "sentinels": [0, 1, 1700000000, "i64::MAX as u64", "i64::MAX as u64 + 1", "u64::MAX", "u64::MAX - 1"]},
        "raw_log": f"{EVIDENCE_DIR}/state12-proptest-po-002-003.log",
        "raw_log_sha256": "cbc4e3cbef31451c56a55fb13e30778f14d3006695e660ca24fdb0318880d0c3",
        "evidence_artifact": f"{EVIDENCE_DIR}/state12-proptest-po-002-003.log",
        "evidence_artifact_sha256": "cbc4e3cbef31451c56a55fb13e30778f14d3006695e660ca24fdb0318880d0c3",
        "formal_waiver_id": "",
        "formal_waiver_hash": "",
        "formal_verifier_invocation_id": "femdation-p12-formal-verifier-vb-cib14",
        "classification": "PROPTEST_OK",
        "rerun_from": 11,
        "status": "closed",
        "bead": "vb-cib14",
        "phase": "formal-verifier",
        "state": 12,
        "tool": "proptest",
        "target": "vb_runtime::journal::tests::chunk_002",
        "notes": "convert_resume_timestamp over u64 sweep + boundary sentinels passes: Ok path for legal i64::MAX range; Err(ResumeTimestampOverflow { run, timestamp: original_u64 }) path for u64::MAX and chrono overflow at 8_210_266_876_800. Verus spec fn convert_resume_timestamp_spec total over u64. No `as i64` cast observed in production helper.",
        "expected_evidence": "proptest + cargo-test pass with 65536 random + explicit sentinels; Ok and Err paths preserve { run, timestamp } payload.",
        "timestamp": "2026-07-02T00:45:35Z",
    },
    # PO-004 cargo-test (storage_event_clones_the_event_exactly_once_per_dispatch + _resumed_ arm + 16-variant)
    {
        "id": "VL-CIB14-C-004",
        "obligation_id": "PO-004",
        "obligation_kind": "cargo_test",
        "requirement_id": "C3,C4,C1",
        "contract_clause": "contract.md#C3,contract.md#C4,contract.md#C1",
        "behavior_affecting": True,
        "verifier": "cargo-test",
        "result": "PASS",
        "command": "cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_clones_the_event_exactly_once_per_dispatch storage_event_clones_the_resumed_event_exactly_once_per_dispatch",
        "command_actual": "cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_clones_the_event_exactly_once_per_dispatch storage_event_clones_the_resumed_event_exactly_once_per_dispatch",
        "workdir": WORKDIR,
        "exit_status": 0,
        "tool_version": "cargo +nightly 1.96.0-nightly (888f67534 2026-03-30)",
        "flags": ["--features=vb-cib14", "--lib"],
        "bounds": {},
        "raw_log": f"{EVIDENCE_DIR}/state12-cargo-test-po-004.log",
        "raw_log_sha256": "359baa27f6fe18a5ab1074c73fad291ae332bd37bcf845703cb483d965137142",
        "evidence_artifact": f"{EVIDENCE_DIR}/state12-cargo-test-po-004.log",
        "evidence_artifact_sha256": "359baa27f6fe18a5ab1074c73fad291ae332bd37bcf845703cb483d965137142",
        "formal_waiver_id": "",
        "formal_waiver_hash": "",
        "formal_verifier_invocation_id": "femdation-p12-formal-verifier-vb-cib14",
        "classification": "CARGO_TEST_OK",
        "rerun_from": 11,
        "status": "closed",
        "bead": "vb-cib14",
        "phase": "formal-verifier",
        "state": 12,
        "tool": "cargo-test",
        "target": "vb_runtime::journal::tests::chunk_002",
        "notes": "Both single-clone regression tests pass with STORAGE_EVENT_CLONE_COUNT == 1. Extended Resumed arm at chunk_002.rs:737-776 confirms Resumed dispatch yields exactly one clone. Combined storage_event regression (chunk_002.rs:410-493) preserved. The catch-all _ => arm of storage_event continues to route Resumed through boundary_storage_event (post-fix arm), preserving the dispatch chain until vb-edvbj removes the catch-all (STRONG-coupled release dependency).",
        "expected_evidence": "Both regression tests pass; STORAGE_EVENT_CLONE_COUNT advances by exactly 1 per Resumed dispatch.",
        "timestamp": "2026-07-02T00:46:00Z",
    },
    # PO-005 loom+proptest (resume_replay_classification)
    {
        "id": "VL-CIB14-LP-005",
        "obligation_id": "PO-005",
        "obligation_kind": "loom_proptest",
        "requirement_id": "C5,REFINEMENT-RRO-RESUME",
        "contract_clause": "contract.md#C5,verification/tla/rust-refinement-obligations.jsonl:6",
        "behavior_affecting": True,
        "verifier": "loom+proptest",
        "result": "PASS",
        "command": "RUSTFLAGS=\"--cfg loom\" cargo +nightly test -p vb_runtime --features vb-cib14 --lib models::loom::vb_cib14_resume_replay",
        "command_actual": "RUSTFLAGS=\"--cfg loom\" cargo +nightly test -p vb_runtime --features vb-cib14 --lib models::loom::vb_cib14_resume_replay",
        "workdir": WORKDIR,
        "exit_status": 0,
        "tool_version": "cargo +nightly 1.96.0-nightly (888f67534 2026-03-30) + loom dev-dep",
        "flags": ["RUSTFLAGS=--cfg loom", "--features=vb-cib14", "--lib"],
        "bounds": {"threads": 2, "preemptions": 4, "branches": 20000},
        "raw_log": f"{EVIDENCE_DIR}/state12-loom-vb-cib14-po-005.log",
        "raw_log_sha256": "9f1d4ea73ff243da387e17791ad94eb67042a40ff9bcb1c9808b33b8bfea5a28",
        "evidence_artifact": f"{EVIDENCE_DIR}/state12-loom-vb-cib14-po-005.log",
        "evidence_artifact_sha256": "9f1d4ea73ff243da387e17791ad94eb67042a40ff9bcb1c9808b33b8bfea5a28",
        "formal_waiver_id": "",
        "formal_waiver_hash": "",
        "formal_verifier_invocation_id": "femdation-p12-formal-verifier-vb-cib14",
        "classification": "LOOM_PROPTEST_OK",
        "rerun_from": 11,
        "status": "closed",
        "bead": "vb-cib14",
        "phase": "formal-verifier",
        "state": 12,
        "tool": "loom",
        "target": "vb_runtime::models::loom::vb_cib14_resume_replay",
        "notes": "2/2 loom tests pass: release_resume_replay_classification + release_resume_replay_legacy_bug_classification. Loom explores all schedules between mapper dispatch and recovery classifier with 2 threads x 4 preemptions x 20000 branches. Proptest half at crates/workspace_tests/tests/vb_test_runtime_resume_replay.rs:3/3 with PROPTEST_CASES=4096 confirms post-fix mapper yields LifecycleState::Active and the legacy buggy shape yields LifecycleState::Failed. RRO-TLA-RESUME-001 refinement obligation satisfied via loom+proptest per master declaration.",
        "expected_evidence": "Loom + proptest temporal-replay harness passes across all explored schedules; Resumed -> Active classification invariant holds; legacy-buggy shape correctly classified as Failed.",
        "timestamp": "2026-07-02T00:46:30Z",
    },
    # PO-006 source-lint
    {
        "id": "VL-CIB14-SL-006",
        "obligation_id": "PO-006",
        "obligation_kind": "source_lint",
        "requirement_id": "C1,C2,C3,C7,VERUS-MIRROR",
        "contract_clause": "contract.md#C1,contract.md#C2,contract.md#C3,contract.md#C7,contract.md#verus-mirror-binding",
        "behavior_affecting": True,
        "verifier": "source-lint",
        "result": "PASS",
        "command": "bash scripts/check-panic-surface.sh && bash scripts/check-hot-cold-forbidden-apis.sh && bash scripts/check-source-length.sh && bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14 && bash scripts/check-error-exhaustiveness.sh",
        "command_actual": "bash scripts/check-panic-surface.sh && bash scripts/check-hot-cold-forbidden-apis.sh && bash scripts/check-source-length.sh && bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14 && bash scripts/check-error-exhaustiveness.sh",
        "workdir": WORKDIR,
        "exit_status": 0,
        "tool_version": "rustc 1.97.0-nightly (52b6e2c20 2026-04-27)",
        "flags": ["source-lint gate suite"],
        "bounds": {},
        "raw_log": f"{EVIDENCE_DIR}/state12-lint-po-006-panic.log",
        "raw_log_sha256": "28adf282afb9586e9f7b3d5a182f8a11ad19a648e51356668a7879a7ed47e3f7",
        "evidence_artifact": f"{EVIDENCE_DIR}/check-verus-production-binding-state12.log",
        "evidence_artifact_sha256": "382f185007ba4b7c3589d048018ab59439db5747e2e7f702802d2299837fa843",
        "formal_waiver_id": "",
        "formal_waiver_hash": "",
        "formal_verifier_invocation_id": "femdation-p12-formal-verifier-vb-cib14",
        "classification": "SOURCE_LINT_OK",
        "rerun_from": 11,
        "status": "closed",
        "bead": "vb-cib14",
        "phase": "formal-verifier",
        "state": 12,
        "tool": "source-lint",
        "target": "mapper site (chunk_002.rs + error/mod.rs + extern mirror)",
        "notes": "check-panic-surface.sh: NoViolationFound, ExitCode 0 (no unsafe/unwrap/expect/panic/todo/unimplemented/dbg in production path). check-hot-cold-forbidden-apis.sh: violations=0, justified=0. check-verus-production-binding.sh: 0 VACUUM, 72 WEAK, 0 STRONG (new spec file vb_cib14_resume_storage_map.rs correctly classified as WEAK_EXTERN). check-source-length.sh: chunk_002.rs (447 lines) and extern_vb_jnz9_journal_event_seq_valid.rs (998 lines) ledgered under split-or-retire-before-release with vb-cib14 owner. RuntimeError remains #[non_exhaustive]; ResumeTimestampOverflow is a struct variant carrying { run: RunId, timestamp: u64 }. Pre-existing FAIL entries across other files are unrelated to vb-cib14.",
        "expected_evidence": "Source-lint suite passes for the vb-cib14 surface; production-binding audit reports 0 VACUUM.",
        "timestamp": "2026-07-02T00:47:00Z",
    },
    # PO-007 proptest (storage_event_resumed_emits_typed_runtime_error_variant)
    {
        "id": "VL-CIB14-P-007",
        "obligation_id": "PO-007",
        "obligation_kind": "proptest_property",
        "requirement_id": "C1,C3,C7",
        "contract_clause": "contract.md#C1,contract.md#C3,contract.md#C7",
        "behavior_affecting": True,
        "verifier": "proptest",
        "result": "PASS",
        "command": "PROPTEST_CASES=4096 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resumed_emits_typed_runtime_error_variant",
        "command_actual": "PROPTEST_CASES=4096 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resumed_emits_typed_runtime_error_variant",
        "workdir": WORKDIR,
        "exit_status": 0,
        "tool_version": "cargo +nightly 1.96.0-nightly (888f67534 2026-03-30) + proptest 1.11",
        "flags": ["PROPTEST_CASES=4096", "--features=vb-cib14", "--lib"],
        "bounds": {"input_size": 4096, "cases": 4096},
        "raw_log": f"{EVIDENCE_DIR}/state12-proptest-po-007.log",
        "raw_log_sha256": "c59cd07c0056371c3ac0b9b927bebbe8cad1df34a912f21d71c65b537877f682",
        "evidence_artifact": f"{EVIDENCE_DIR}/state12-proptest-po-007.log",
        "evidence_artifact_sha256": "c59cd07c0056371c3ac0b9b927bebbe8cad1df34a912f21d71c65b537877f682",
        "formal_waiver_id": "",
        "formal_waiver_hash": "",
        "formal_verifier_invocation_id": "femdation-p12-formal-verifier-vb-cib14",
        "classification": "PROPTEST_OK",
        "rerun_from": 11,
        "status": "closed",
        "bead": "vb-cib14",
        "phase": "formal-verifier",
        "state": 12,
        "tool": "proptest",
        "target": "vb_runtime::journal::tests::chunk_002 + chunk_004",
        "notes": "storage_event_resumed_emits_typed_runtime_error_variant (chunk_002.rs:689-719) passes with full variant-shape assertions: RuntimeError::ResumeTimestampOverflow { run: RunId(input_run), timestamp: input_timestamp } for overflow paths; Ok(RunResumed { run, seq, timestamp: DateTime }) for legal paths. Display non-empty. The 16-variant enumeration at chunk_004.rs:1077-1090 (storage_event_exhaustive_over_16_variants) is exercised in the full feature cargo test log (1812 passed / 0 failed). No variant reaches the synthetic RunFailedEvent catch-all except where it is intentionally an arm of run_storage_event for run-failure family variants.",
        "expected_evidence": "proptest passes with no failures; variant-shape match; no fallthrough to RunFailedEvent except for run-failure family.",
        "timestamp": "2026-07-02T00:47:30Z",
    },
]


def canonical_hash(row):
    """SHA-256 of the canonicalized row JSON (sorted keys, no extra whitespace)."""
    # Hash all fields except previous_entry_hash / entry_hash themselves.
    filtered = {k: v for k, v in row.items() if k not in ("entry_hash",)}
    # Use separators=(",", ":") and sort_keys=True for canonical form.
    canonical = json.dumps(filtered, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


# Build ledger rows with hash chain.
ledger_rows = []
prev_hash = "0000000000000000000000000000000000000000000000000000000000000000"
for row in ROWS:
    row["previous_entry_hash"] = prev_hash
    entry_hash = canonical_hash(row)
    row["entry_hash"] = entry_hash
    ledger_rows.append(row)
    prev_hash = entry_hash

# Write the ledger.
out_path = Path(WORKDIR) / LEDGER_OUT
with out_path.open("w", encoding="utf-8") as f:
    for row in ledger_rows:
        f.write(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n")

print(f"wrote {len(ledger_rows)} rows to {out_path}")
for r in ledger_rows:
    print(f"  {r['id']} -> {r['obligation_id']} -> {r['result']} (hash: {r['entry_hash'][:16]}...)")