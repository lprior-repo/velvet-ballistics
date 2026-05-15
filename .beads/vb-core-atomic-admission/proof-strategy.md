# Proof Strategy: vb-core-atomic-admission

bead_id: `vb-core-atomic-admission`
state: 4 proof planning
attempt: 3-of-7
status: PLANNED_AFTER_STATE_3_REPAIR

## Scope

Planning refresh consumes repaired State 3 artifacts and State 6 rejection artifacts only. It writes no production code, tests, proof/model/harness/spec files, dependency files, or source checkout files.

## Inputs Read

- `.beads/vb-core-atomic-admission/contract.md`
- `.beads/vb-core-atomic-admission/verification-layers.md`
- `.beads/vb-core-atomic-admission/proof-obligations.jsonl`
- `.beads/vb-core-atomic-admission/traceability-matrix.jsonl`
- `.beads/vb-core-atomic-admission/delivery-scope.jsonl`
- `.beads/vb-core-atomic-admission/codebase-map.md`
- `.beads/vb-core-atomic-admission/proof-review.md`
- `.beads/vb-core-atomic-admission/proof-findings.jsonl`
- `.beads/vb-core-atomic-admission/proof-repair-guide.md`
- `.beads/vb-core-atomic-admission/contract-verification-review.md`
- Prior `.beads/vb-core-atomic-admission/proof-evidence.md` and `proof-writer-report.md` as rejected context only.

## Discovery Commands

- `pwd -P` exited 0 and returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission`.
- `test -s ".beads/vb-core-atomic-admission/contract.md" && test -s ".beads/vb-core-atomic-admission/traceability-matrix.jsonl" && test -s ".beads/vb-core-atomic-admission/delivery-scope.jsonl"` exited 0.
- `/usr/bin/rg -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/vb_storage/src crates/vb_runtime/src crates/velvet_ballastics/src crates/velvet_ballastics/tests/admission_evidence_integration verification/tla verification/verus` exited 0; output was large and persisted by the tool at `/home/lewis/.local/share/opencode/tool-output/tool_e2d6758c70016F4jHexEuCyf3c`.
- `/usr/bin/rg -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/vb_storage/src crates/vb_runtime/src crates/velvet_ballastics/src crates/velvet_ballastics/tests/admission_evidence_integration verification/tla verification/verus` exited 0.

No scoped discovery command was blocked.

## Risk Classification

- Temporal/durability: required. Use TLA+ for all-or-none commit, before-ack ordering, failure injection, restart/readback, deadlock checking, and post-commit readback liveness.
- Rust-local pure invariants: required. Use Verus for input coherence, sequence binding, strict artifact tag discrimination, narrowed index precondition decomposition, and narrowed error taxonomy totality.
- Production realization of sequence binding: high risk, but current exact Kani harness is absent. Keep `KANI-PROP-007` as a waiver with owner/expiry/compensating evidence until State 8 can create an exact harness.
- Hostile malformed payload bytes: high risk, but current exact cargo-fuzz target is absent. Keep `FUZZ-ART-008` as a waiver with owner/expiry/compensating evidence until State 8 can create an exact target.
- Codec/readback UB: Miri is planned only if codec/raw-byte strict artifact paths change.
- Fail-closed implementation behavior: require integration scenarios, static scan, and mutation; Verus taxonomy alone must not claim production Result propagation.
- Concurrency/Loom: not applicable unless implementation introduces concurrent shared admission state beyond durable batch ordering.
- Dependencies/supply chain: no dependency files are in repaired scope; no new lane required unless implementation changes dependency files.
- Performance: not applicable; no speed claim exists.

## Rejection Repairs Reflected

- `TLA-ATOM-001` now explicitly requires deadlock checking not be disabled and requires TLC to check `EventuallyReadableAfterCommit`.
- `VERUS-IDX-005` is narrowed to pure precondition decomposition unless State 5 strengthens the model with concrete index key/value derivation.
- `VERUS-ERR-006` is narrowed to taxonomy totality unless State 5 adds a pure admission outcome transition proving no silent success.
- `KANI-PROP-007` and `FUZZ-ART-008` are explicit waiver rows with owner, reason, limitation, expiry, and compensating evidence.
- `ERR-INVALID-015` through `ERR-INDEX-022` are planned as separate per-variant executable `moon ci` evidence rows.

## Planned Artifacts

- `verification/tla/AtomicAcceptedRunAdmission.tla`
- `verification/tla/AtomicAcceptedRunAdmission.cfg`
- `verification/verus/accepted_run_atomic_admission.rs`
- Later implementation/test/formal evidence reports referenced by `proof-obligations.planned.jsonl`.

## Reviewer Focus

Review should reject if any row claims pass evidence, if TLA+ omits deadlock/readback liveness obligations, if Verus claims overstate production behavior, or if waived high-risk Kani/fuzz lanes lack owner/expiry/compensating evidence.
