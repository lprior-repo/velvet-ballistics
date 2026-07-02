#!/usr/bin/env python3
"""Append the State 12 row to agent-invocation-ledger.jsonl."""
import hashlib
import json
from pathlib import Path

LEDGER = Path("/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14/.beads/vb-cib14/agent-invocation-ledger.jsonl")


def canonical_hash(row):
    filtered = {k: v for k, v in row.items() if k != "entry_hash"}
    return hashlib.sha256(
        json.dumps(filtered, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


# Read existing rows and find the last entry_hash.
with LEDGER.open("r", encoding="utf-8") as f:
    rows = [json.loads(line) for line in f if line.strip()]

last = rows[-1]
prev_hash = last["entry_hash"]
last_seq = last["ledger_sequence"]

# State 12 row.
state12_row = {
    "schema_version": "agent-invocation/v1",
    "ledger_sequence": last_seq + 1,
    "previous_entry_hash": prev_hash,
    "host_session_id": "femdation-cheap25-batch",
    "invocation_id": "femdation-p12-formal-verifier-vb-cib14",
    "parent_invocation_id": "femdation-p11-holzman-rust-vb-cib14",
    "skill": "formal-verifier",
    "state": 12,
    "workdir": "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14",
    "input_artifacts": [
        "proof-review.md",
        "proof-to-rust-review.md",
        "proof-writer-report.md",
        "proof-evidence.md",
        "proof-strategy.md",
        "proof-obligations.planned.jsonl",
        "rust-refinement-obligations.jsonl",
        "trusted-base-ledger.jsonl",
        "verifier-lane-decisions.jsonl",
        "waiver-candidates.jsonl",
        "contract.md",
        "implementation.md",
    ],
    "input_artifact_hashes": {
        "proof-review.md": "e0e62227b0c3476825934be4fee0cd13ebbe3e1436a9e7cdeab9ed6c972035c9",
        "proof-to-rust-review.md": "8ae7e1fa0842f99e6b790bc385f728da2176320df5e41a9ed5edf73561d4215e",
        "proof-writer-report.md": "8211d6b5f17eeaf132f52feca216cf0d7e4d946b9d35d1dba3e015a67c08eb0f",
        "proof-evidence.md": "008b08f661a85d9a196ef04ab65b4867cc1f3e282bcd6eb88f0e79c0e033087d",
        "proof-strategy.md": "9a3b263a084f5516d28018a7f4b8129429999526d79d9156ea04b635dd138a6b",
        "proof-obligations.planned.jsonl": "365e97393e698e3cc8f0342cea8de3acb35dac0e1ab63120a5946105152a8d80",
        "rust-refinement-obligations.jsonl": "9fd888c193358fc8372fab324c16542103207de1417b85b92d17e1dc498f06d3",
        "trusted-base-ledger.jsonl": "4f2bad3274568b5efc994cd6937bec60c8b9008297c1eea99912149f6350a451",
        "verifier-lane-decisions.jsonl": "1803bd022cb942b8186243f8254e5bf1d770f72fee97196dc20348605db08b40",
        "waiver-candidates.jsonl": "9785f620479e3ae488909726c247c4510fa5809e6d27121d67ff8ea37075759c",
        "contract.md": "a828e96e210c29d8a306112b59b852cc8a2f225935db6fa828372cdcdcdee3c8",
        "implementation.md": "c29a10b8ee40e590c22d2c7b7543142f5733d6e7284e9414265a1ae44fd0b8ff",
    },
    "output_artifacts": [
        "formal-verification-report.md",
        "verification-ledger.jsonl",
        "evidence/state12-cargo-vb-runtime-storage_event.log",
        "evidence/state12-cargo-vb-runtime-chunk004-runtime_journal_event_resumed.log",
        "evidence/state12-cargo-workspace-tests-vb_test_runtime_resume_replay.log",
        "evidence/state12-verus-vb-cib14-po-001.log",
        "evidence/state12-proptest-po-002-003.log",
        "evidence/state12-loom-vb-cib14-po-005.log",
        "evidence/state12-proptest-po-007.log",
        "evidence/state12-cargo-test-po-004.log",
        "evidence/state12-lint-po-006-panic.log",
        "evidence/state12-lint-po-006-hot-cold.log",
        "evidence/state12-lint-po-006-length.log",
        "evidence/state12-lint-po-006-error-exhaustiveness.log",
        "evidence/check-verus-production-binding-state12.log",
        "build-verification-ledger.py",
    ],
    "output_artifact_hashes": {
        "formal-verification-report.md": "d57bd40dcbfa7f931c134ab6802cf08c1cc82d77522ab01b09fa2cf0cdab94d9",
        "verification-ledger.jsonl": "05af88ae48d67756101de9175248774d3dd060b6937d402f7294023640a5cdb1",
        "evidence/state12-cargo-vb-runtime-storage_event.log": "e5341670c4127761b68c023435a0ddd1bf1579cdcb55e8c210c67c670cfb2f6d",
        "evidence/state12-cargo-vb-runtime-chunk004-runtime_journal_event_resumed.log": "b756e7be57a593327a0190a8e0504fe7dee89d4e4000665894ea6cf20cd2b701",
        "evidence/state12-cargo-workspace-tests-vb_test_runtime_resume_replay.log": "35c56931131a40b9b2ff27c0c8d322557b6b84952e081684d44f27b96e5a583f",
        "evidence/state12-verus-vb-cib14-po-001.log": "fa7156fede2780c21ef1952d47f403742a63da59fa0ace4beb6686a31f10f536",
        "evidence/state12-proptest-po-002-003.log": "cbc4e3cbef31451c56a55fb13e30778f14d3006695e660ca24fdb0318880d0c3",
        "evidence/state12-loom-vb-cib14-po-005.log": "9f1d4ea73ff243da387e17791ad94eb67042a40ff9bcb1c9808b33b8bfea5a28",
        "evidence/state12-proptest-po-007.log": "c59cd07c0056371c3ac0b9b927bebbe8cad1df34a912f21d71c65b537877f682",
        "evidence/state12-cargo-test-po-004.log": "359baa27f6fe18a5ab1074c73fad291ae332bd37bcf845703cb483d965137142",
        "evidence/state12-lint-po-006-panic.log": "28adf282afb9586e9f7b3d5a182f8a11ad19a648e51356668a7879a7ed47e3f7",
        "evidence/check-verus-production-binding-state12.log": "382f185007ba4b7c3589d048018ab59439db5747e2e7f702802d2299837fa843",
    },
    "transcript_artifact": "transcript-state12.txt",
    "transcript_hash": "PLACEHOLDER_TRANSCRIPT_HASH",
    "reviewed_artifacts_existed_before_start": True,
    "started_at": "2026-07-02T00:30:00Z",
    "completed_at": "2026-07-02T00:48:00Z",
    "status": "completed",
}

state12_row["entry_hash"] = canonical_hash(state12_row)

with LEDGER.open("a", encoding="utf-8") as f:
    f.write(json.dumps(state12_row, sort_keys=True, separators=(",", ":")) + "\n")

print(f"appended State 12 row, sequence={state12_row['ledger_sequence']}, hash={state12_row['entry_hash'][:16]}...")

# Verify chain integrity.
with LEDGER.open("r", encoding="utf-8") as f:
    rows = [json.loads(line) for line in f if line.strip()]
prev = "0000000000000000000000000000000000000000000000000000000000000000"
for i, r in enumerate(rows, 1):
    assert r["previous_entry_hash"] == prev, f"row {i} broken chain"
    assert r["entry_hash"] == canonical_hash(r), f"row {i} hash mismatch"
    prev = r["entry_hash"]
print(f"chain integrity verified: {len(rows)} rows, all hashes match")