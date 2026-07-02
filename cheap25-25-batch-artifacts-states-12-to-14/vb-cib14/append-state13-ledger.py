#!/usr/bin/env python3
"""Append the State 13 row to agent-invocation-ledger.jsonl."""
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

# Compute sha256 of the new black-hat review.
black_hat_sha = hashlib.sha256(
    Path("/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14/.beads/vb-cib14/black-hat-review.md").read_bytes()
).hexdigest()

state13_row = {
    "schema_version": "agent-invocation/v1",
    "ledger_sequence": last_seq + 1,
    "previous_entry_hash": prev_hash,
    "host_session_id": "femdation-cheap25-batch",
    "invocation_id": "femdation-p13-black-hat-reviewer-vb-cib14",
    "parent_invocation_id": "femdation-p12-formal-verifier-vb-cib14",
    "skill": "black-hat-reviewer",
    "state": 13,
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
        "formal-verification-report.md",
        "verification-ledger.jsonl",
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
        "formal-verification-report.md": "d57bd40dcbfa7f931c134ab6802cf08c1cc82d77522ab01b09fa2cf0cdab94d9",
        "verification-ledger.jsonl": "05af88ae48d67756101de9175248774d3dd060b6937d402f7294023640a5cdb1",
    },
    "output_artifacts": [
        "black-hat-review.md",
    ],
    "output_artifact_hashes": {
        "black-hat-review.md": black_hat_sha,
    },
    "transcript_artifact": "transcript-state13.txt",
    "transcript_hash": "PLACEHOLDER_TRANSCRIPT_HASH",
    "reviewed_artifacts_existed_before_start": True,
    "started_at": "2026-07-02T00:48:00Z",
    "completed_at": "2026-07-02T01:00:00Z",
    "status": "completed",
}

state13_row["entry_hash"] = canonical_hash(state13_row)

with LEDGER.open("a", encoding="utf-8") as f:
    f.write(json.dumps(state13_row, sort_keys=True, separators=(",", ":")) + "\n")

print(f"appended State 13 row, sequence={state13_row['ledger_sequence']}, hash={state13_row['entry_hash'][:16]}...")

# Verify chain integrity.
with LEDGER.open("r", encoding="utf-8") as f:
    rows = [json.loads(line) for line in f if line.strip()]
prev = "0000000000000000000000000000000000000000000000000000000000000000"
for i, r in enumerate(rows, 1):
    assert r["previous_entry_hash"] == prev, f"row {i} broken chain"
    assert r["entry_hash"] == canonical_hash(r), f"row {i} hash mismatch"
    prev = r["entry_hash"]
print(f"chain integrity verified: {len(rows)} rows, all hashes match")