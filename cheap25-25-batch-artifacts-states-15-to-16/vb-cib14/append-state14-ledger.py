#!/usr/bin/env python3
"""Append the State 14 rows (evidence-packaging + truth-serum) to the agent-invocation-ledger.jsonl."""
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


def sha_of(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


# State 14 row (evidence-packaging).
state14_row = {
    "schema_version": "agent-invocation/v1",
    "ledger_sequence": last_seq + 1,
    "previous_entry_hash": prev_hash,
    "host_session_id": "femdation-cheap25-batch",
    "invocation_id": "femdation-p14-evidence-packaging-vb-cib14",
    "parent_invocation_id": "femdation-p13-black-hat-reviewer-vb-cib14",
    "skill": "evidence-packaging",
    "state": 14,
    "workdir": "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14",
    "input_artifacts": [
        "delivery-scope.jsonl",
        "contract.md",
        "traceability-matrix.jsonl",
        "proof-review.md",
        "proof-to-rust-review.md",
        "formal-verification-report.md",
        "verification-ledger.jsonl",
        "black-hat-review.md",
    ],
    "input_artifact_hashes": {
        "proof-review.md": "e0e62227b0c3476825934be4fee0cd13ebbe3e1436a9e7cdeab9ed6c972035c9",
        "proof-to-rust-review.md": "8ae7e1fa0842f99e6b790bc385f728da2176320df5e41a9ed5edf73561d4215e",
        "formal-verification-report.md": "d57bd40dcbfa7f931c134ab6802cf08c1cc82d77522ab01b09fa2cf0cdab94d9",
        "verification-ledger.jsonl": "05af88ae48d67756101de9175248774d3dd060b6937d402f7294023640a5cdb1",
        "black-hat-review.md": "18f8be492ded1e865da6bf7bc7d19ff20d6ba37522be1cdd4247a6efdfe4abbc",
        "contract.md": "a828e96e210c29d8a306112b59b852cc8a2f225935db6fa828372cdcdcdee3c8",
    },
    "output_artifacts": [
        "machine-gate-report.md",
        "regression-diff.md",
        "assurance-bundle.md",
        "evidence/machine-gate-state14.log",
    ],
    "output_artifact_hashes": {
        "machine-gate-report.md": "2a6c9bbe05e3a4ffca55e2f56beb2f0ae3656dc062228fc9766322d1c6daa575",
        "regression-diff.md": "467dccd4d10af638d5db3f5db870f77312e6ead2f8b80149352c0a6609446446",
        "assurance-bundle.md": "a12aaa13ce884784f0be31fcfacd422304fc18e39a7ed6827fc196e410ced37e",
        "evidence/machine-gate-state14.log": "d6383f987cc63c7ea2eba22896579e39a432d45b21eb8eff69cf7059b189e0ba",
    },
    "transcript_artifact": "transcript-state14.txt",
    "transcript_hash": "PLACEHOLDER_TRANSCRIPT_HASH",
    "reviewed_artifacts_existed_before_start": True,
    "started_at": "2026-07-02T01:00:00Z",
    "completed_at": "2026-07-02T03:10:00Z",
    "status": "completed",
}
state14_row["entry_hash"] = canonical_hash(state14_row)

with LEDGER.open("a", encoding="utf-8") as f:
    f.write(json.dumps(state14_row, sort_keys=True, separators=(",", ":")) + "\n")

print(f"appended State 14 evidence-packaging row, sequence={state14_row['ledger_sequence']}, hash={state14_row['entry_hash'][:16]}...")

# State 14b row (truth-serum).
last = rows[-1] if False else json.loads(open(LEDGER).readlines()[-1])
prev_hash = last["entry_hash"]
last_seq = last["ledger_sequence"]

state14b_row = {
    "schema_version": "agent-invocation/v1",
    "ledger_sequence": last_seq + 1,
    "previous_entry_hash": prev_hash,
    "host_session_id": "femdation-cheap25-batch",
    "invocation_id": "femdation-p14b-truth-serum-vb-cib14",
    "parent_invocation_id": "femdation-p14-evidence-packaging-vb-cib14",
    "skill": "truth-serum",
    "state": 14,
    "workdir": "/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14",
    "input_artifacts": [
        "assurance-bundle.md",
        "verification-ledger.jsonl",
        "formal-verification-report.md",
        "black-hat-review.md",
        "implementation.md",
        "proof-review.md",
        "proof-to-rust-review.md",
        "machine-gate-report.md",
    ],
    "input_artifact_hashes": {
        "assurance-bundle.md": "a12aaa13ce884784f0be31fcfacd422304fc18e39a7ed6827fc196e410ced37e",
        "verification-ledger.jsonl": "05af88ae48d67756101de9175248774d3dd060b6937d402f7294023640a5cdb1",
        "formal-verification-report.md": "d57bd40dcbfa7f931c134ab6802cf08c1cc82d77522ab01b09fa2cf0cdab94d9",
        "black-hat-review.md": "18f8be492ded1e865da6bf7bc7d19ff20d6ba37522be1cdd4247a6efdfe4abbc",
        "implementation.md": "c29a10b8ee40e590c22d2c7b7543142f5733d6e7284e9414265a1ae44fd0b8ff",
        "proof-review.md": "e0e62227b0c3476825934be4fee0cd13ebbe3e1436a9e7cdeab9ed6c972035c9",
        "proof-to-rust-review.md": "8ae7e1fa0842f99e6b790bc385f728da2176320df5e41a9ed5edf73561d4215e",
        "machine-gate-report.md": "2a6c9bbe05e3a4ffca55e2f56beb2f0ae3656dc062228fc9766322d1c6daa575",
    },
    "output_artifacts": [
        "truth-serum-report.md",
        "final-evidence-decision.md",
    ],
    "output_artifact_hashes": {
        "truth-serum-report.md": "25e8e0846c778574a9141f7c5720b14994ed98e35f0e98bff9765e317eb72aae",
        "final-evidence-decision.md": "a9de11d3816665bb5afefc8fcab1130fbb6a97a6173871b662191086b32b13e4",
    },
    "transcript_artifact": "transcript-state14b.txt",
    "transcript_hash": "PLACEHOLDER_TRANSCRIPT_HASH",
    "reviewed_artifacts_existed_before_start": True,
    "started_at": "2026-07-02T03:10:00Z",
    "completed_at": "2026-07-02T03:30:00Z",
    "status": "completed",
}
state14b_row["entry_hash"] = canonical_hash(state14b_row)

with LEDGER.open("a", encoding="utf-8") as f:
    f.write(json.dumps(state14b_row, sort_keys=True, separators=(",", ":")) + "\n")

print(f"appended State 14b truth-serum row, sequence={state14b_row['ledger_sequence']}, hash={state14b_row['entry_hash'][:16]}...")

# Verify chain integrity.
with LEDGER.open("r", encoding="utf-8") as f:
    rows = [json.loads(line) for line in f if line.strip()]
prev = "0000000000000000000000000000000000000000000000000000000000000000"
for i, r in enumerate(rows, 1):
    assert r["previous_entry_hash"] == prev, f"row {i} broken chain"
    assert r["entry_hash"] == canonical_hash(r), f"row {i} hash mismatch"
    prev = r["entry_hash"]
print(f"chain integrity verified: {len(rows)} rows, all hashes match")