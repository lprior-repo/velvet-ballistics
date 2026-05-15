# Proof Repair Guide - vb-qi37.4.2

STATUS: REPAIR_REQUIRED

## Required Repairs

1. Update `.beads/vb-qi37.4.2/proof-obligations.jsonl` so `VERUS-ENV-006` names `verification/verus/accepted_envelope_model.rs`, uses checker `verus`, records command `verus verification/verus/accepted_envelope_model.rs`, and points at an existing evidence artifact.
2. Align all executed TLA+/Verus obligation evidence paths with actual files. Either create `.beads/vb-qi37.4.2/tla-report.md` and `.beads/vb-qi37.4.2/verus-report.md`, or update evidence fields to `.beads/vb-qi37.4.2/proof-evidence.md` with section references.
3. Execute or explicitly waive every required planned lane: `PO-007`, `PO-008`, `PO-009`, `PO-010`, `PO-011`, and `PO-012`.
4. For any lane deferred by lifecycle state, make the deferral explicit in the current review target: owner, expiry condition, and compensating evidence. Do not leave required rows as plain `planned` and then request proof approval.
5. Re-run the proof-review verification set after the ledger and evidence repairs.

## Minimum Rerun Targets

```bash
python -c 'import json, pathlib; [json.loads(line) for path in [".beads/vb-qi37.4.2/proof-obligations.jsonl", ".beads/vb-qi37.4.2/proof-obligations.planned.jsonl", ".beads/vb-qi37.4.2/traceability-matrix.jsonl"] for line in pathlib.Path(path).read_text().splitlines() if line.strip()]'
TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2/.tmp tlc -metadir .beads/vb-qi37.4.2/review-tlc-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla
verus verification/verus/capability_artifact_model.rs
verus verification/verus/accepted_envelope_model.rs
```

Add the Kani, fuzz, proptest, static scan, mutation, and CI commands once their artifacts are present or waivers are formally recorded.
