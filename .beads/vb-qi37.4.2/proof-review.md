# Proof Review - vb-qi37.4.2

STATUS: APPROVED

Timestamp: `2026-05-15T22:41:50Z`

## Scope

- State: 6 proof-review retry after State 5 attempt 3 repair.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`.
- Source checkout write status: none; `/home/lewis/src/velvet-ballistics` was not written.
- Review-only writes: `.beads/vb-qi37.4.2/proof-review.md`, `.beads/vb-qi37.4.2/proof-findings.jsonl`, and `.beads/vb-qi37.4.2/STATE.md`.
- Reviewed inputs: refreshed `.beads/vb-qi37.4.2/proof-writer-report.md`, `.beads/vb-qi37.4.2/proof-evidence.md`, repaired `.beads/vb-qi37.4.2/proof-obligations.jsonl`, repaired `.beads/vb-qi37.4.2/proof-obligations.planned.jsonl`, `.beads/vb-qi37.4.2/contract.md`, `.beads/vb-qi37.4.2/traceability-matrix.jsonl`, and existing `verification/tla/CapabilityLifecycle.tla`, `verification/tla/*.cfg`, `verification/verus/capability_artifact_model.rs`, `verification/verus/accepted_envelope_model.rs`.

## Findings

- No rejecting proof findings for the State 5 executable proof scope.
- Residual boundaries are informational only: Kani digest, hostile-byte fuzzing, invalid-space proptest, diagnostic mutation, static scan, strict admission tests, and canonical CI remain later-owner evidence obligations and must not be claimed as proof passes before execution or downstream WAIVED/DEFERRED records.

## Validation Evidence

- Isolation command: `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac && test -s ".beads/vb-qi37.4.2/STATE.md"` exited 0.
- JSONL command: `jq -c . ".beads/vb-qi37.4.2/proof-obligations.jsonl" >/dev/null && jq -c . ".beads/vb-qi37.4.2/proof-obligations.planned.jsonl" >/dev/null && jq -c . ".beads/vb-qi37.4.2/traceability-matrix.jsonl" >/dev/null && jq -c . ".beads/vb-qi37.4.2/proof-findings.jsonl" >/dev/null` exited 0 before this rewrite.
- Reviewed ledger facts: `proof-obligations.jsonl` has 12 rows and all contract-time rows are `status:"planned"`; `proof-obligations.planned.jsonl` has 19 rows with statuses limited to `planned` and `not_applicable`.
- Discovery scan found TLA+ invariants/properties and Verus proof functions; no hidden Kani, Loom, fuzz, or proptest executable pass claim was found in the State 5 proof package.

## Verifier Reruns

- `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-review-retry-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla`: exit 0; no TLC error; `478 states generated, 220 distinct states found, 0 states left on queue`.
- `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-review-retry-gate -config verification/tla/CapabilityLifecycleGateMismatch.cfg verification/tla/CapabilityLifecycle.tla`: exit 0; no TLC error; same state counts.
- `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-review-retry-excess -config verification/tla/CapabilityLifecycleExcessGrant.cfg verification/tla/CapabilityLifecycle.tla`: exit 0; no TLC error; same state counts.
- `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-review-retry-exact -config verification/tla/CapabilityLifecycleExactProfile.cfg verification/tla/CapabilityLifecycle.tla`: exit 0; no TLC error; same state counts.
- `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-review-retry-legacy -config verification/tla/CapabilityLifecycleLegacyBypass.cfg verification/tla/CapabilityLifecycle.tla`: exit 0; no TLC error; same state counts.
- `TMPDIR=target/tmp verus verification/verus/capability_artifact_model.rs`: exit 0; `verification results:: 8 verified, 0 errors`.
- `TMPDIR=target/tmp verus verification/verus/accepted_envelope_model.rs`: exit 0; `verification results:: 8 verified, 0 errors`.

## Obligation Review

- `PO-001` through `PO-004` / `TLA-ADMIT-001`, `TLA-GATE-002`, `TLA-CAP-003`, `TLA-BYPASS-004`: approved for finite safety-model claims only over the stated bounds. They cover denial-before-allocation, gate mismatch denial, capability-count mismatch denial, and legacy bypass denial.
- `PO-005` / `VERUS-CAP-005`: approved for decoded-value exact capability predicate claims only. It excludes Fjall I/O, postcard decoding, runtime constructors, CLI, and IPC.
- `PO-006` / `VERUS-ENV-006`: approved for decoded accepted-envelope predicate claims only. It excludes raw byte decoding, storage I/O, wall-clock reads, and production wiring.
- `PO-007`, `PO-008`, `PO-009`, `PO-011`, and `PO-012`: accepted only as planned downstream evidence-policy rows, not proof passes and not State 5 waivers.
- `PO-010` and `PO-019`: later owner-state obligations; no State 5 proof pass is claimed.

## Decision

The repaired State 5 proof package is approved for its narrow executable TLA+/Verus scope. Downstream states must preserve the evidence boundaries and cannot claim Kani, fuzz, proptest, mutation, static-scan, strict admission test, or canonical CI evidence until those gates run or formal downstream WAIVED/DEFERRED records are produced.
