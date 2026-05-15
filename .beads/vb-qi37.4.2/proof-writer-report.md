# Proof Writer Report - vb-qi37.4.2

## Identification

- Bead: `vb-qi37.4.2`
- State: 5 proof/model/harness writing repair
- Attempt: 3 of 7
- Timestamp: `2026-05-15T22:33:47Z`
- Isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`
- Source checkout write status: none; `/home/lewis/src/velvet-ballistics` was not written.
- Scope constraint: verification artifacts and bead evidence only; no production source, test, dependency, CI, or source-checkout edits.

## Inputs Read

- `.beads/vb-qi37.4.2/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.4.2/proof-obligations.jsonl`
- `.beads/vb-qi37.4.2/proof-strategy.md`
- `.beads/vb-qi37.4.2/proof-plan-review-input.md`
- `.beads/vb-qi37.4.2/contract.md`
- `.beads/vb-qi37.4.2/traceability-matrix.jsonl`
- Prior State 6 rejection artifacts: `.beads/vb-qi37.4.2/proof-review.md`, `.beads/vb-qi37.4.2/proof-findings.jsonl`, `.beads/vb-qi37.4.2/proof-repair-guide.md`, `.beads/vb-qi37.4.2/contract-verification-review.md`

## Changed Artifacts

- Refreshed `.beads/vb-qi37.4.2/proof-writer-report.md` for State 5 attempt 3 after State 4 plan repair.
- Refreshed `.beads/vb-qi37.4.2/proof-evidence.md` with `TMPDIR=target/tmp` raw command evidence and planned downstream evidence-policy boundaries.
- Appended `.beads/vb-qi37.4.2/STATE.md` with State 5 attempt 3 transition/completion.

No executable proof logic was weakened. No assumptions, invariants, or contracts were relaxed.

## Obligation Results

| ID | Artifact | Command | Status |
|---|---|---|---|
| `PO-001` / `TLA-ADMIT-001` | `verification/tla/CapabilityLifecycle.tla` with `CapabilityLifecycleAll.cfg` | `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla` | PASS |
| `PO-002` / `TLA-GATE-002` | `verification/tla/CapabilityLifecycle.tla` with `CapabilityLifecycleGateMismatch.cfg` | `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-gate -config verification/tla/CapabilityLifecycleGateMismatch.cfg verification/tla/CapabilityLifecycle.tla` | PASS |
| `PO-003` / `TLA-CAP-003` | `verification/tla/CapabilityLifecycle.tla` with `CapabilityLifecycleExcessGrant.cfg` | `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-excess -config verification/tla/CapabilityLifecycleExcessGrant.cfg verification/tla/CapabilityLifecycle.tla` | PASS |
| `PO-003` / `TLA-CAP-003` | `verification/tla/CapabilityLifecycle.tla` with `CapabilityLifecycleExactProfile.cfg` | `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-exact -config verification/tla/CapabilityLifecycleExactProfile.cfg verification/tla/CapabilityLifecycle.tla` | PASS |
| `PO-004` / `TLA-BYPASS-004` | `verification/tla/CapabilityLifecycle.tla` with `CapabilityLifecycleLegacyBypass.cfg` | `TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-legacy -config verification/tla/CapabilityLifecycleLegacyBypass.cfg verification/tla/CapabilityLifecycle.tla` | PASS |
| `PO-005` / `VERUS-CAP-005` | `verification/verus/capability_artifact_model.rs` | `TMPDIR=target/tmp verus verification/verus/capability_artifact_model.rs` | PASS |
| `PO-006` / `VERUS-ENV-006` | `verification/verus/accepted_envelope_model.rs` | `TMPDIR=target/tmp verus verification/verus/accepted_envelope_model.rs` | PASS |
| `PO-007` | `verification/kani/digest_admission_harness.rs` | planned downstream evidence-policy row; harness absent | PLANNED_POLICY, no Kani pass or contract-time waiver claimed |
| `PO-008` | `fuzz/fuzz_targets/accepted_artifact_envelope.rs` | planned downstream evidence-policy row; target absent | PLANNED_POLICY, no fuzz pass or contract-time waiver claimed |
| `PO-009` | proptest invalid-space lane | planned downstream evidence-policy row; no confirmed target | PLANNED_POLICY, no proptest pass or contract-time waiver claimed |
| `PO-010` | static scan/lint lane | later owner state 8 per repaired plan | NOT_RUN in State 5 |
| `PO-011` | diagnostic mutation lane | planned downstream evidence-policy row until diagnostic tests exist | PLANNED_POLICY, no mutation pass or contract-time waiver claimed |
| `PO-012` | canonical CI | planned downstream evidence-policy row until formal-verifier/landing | PLANNED_POLICY, no CI pass or contract-time waiver claimed |

## Tooling Discovery

| Tool | Status | Evidence |
|---|---|---|
| Java | FOUND | `which java` -> `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java` |
| TLC | FOUND | `which tlc` -> `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc` |
| Verus | FOUND | `which verus` -> `/home/lewis/.local/bin/verus`; `verus --version` -> `0.2026.05.05.d03e906` |
| Kani | FOUND | `cargo kani --version` -> `cargo-kani 0.67.0` |
| cargo-fuzz | FOUND | `cargo fuzz --version` -> `cargo-fuzz 0.13.1` |
| Miri | FOUND | `cargo +nightly miri --version` -> `miri 0.1.0 (e0e95a7187 2026-04-04)` |
| Flux | BLOCKED_TOOLING | `cargo flux --version` failed with `error: no such command: flux`; Flux remains not-applicable per `PO-017` |

## Assumptions And Boundaries

- TLA+ state bounds are `GateCounts={0,2,CanonicalGate}`, `CapabilityCounts=0..2`, and `CanonicalGate=15`.
- TLA+ proves safety only: denied admission does not allocate or journal accepted state; it does not prove liveness, byte decoding, or production storage wiring.
- Verus `capability_artifact_model.rs` proves exact capability predicates on decoded domain values, not Fjall I/O or postcard decoding.
- Verus `accepted_envelope_model.rs` proves decoded accepted-envelope predicates: schema v1, canonical gate count 15, durable flag, non-stale evidence, and accepted proof flags.
- Raw hostile bytes, digest equality over persisted records, production strict-path wiring, diagnostic preservation, integration behavior, mutation resistance, and canonical CI remain owned by later planned downstream evidence-policy lanes. No pass or contract-time waiver is claimed for those lanes here.

## Reviewer Guidance

- Review `.beads/vb-qi37.4.2/proof-evidence.md` attempt 3 command sections for exact exit evidence.
- Treat `PO-001` through `PO-006` as executable State 5 proof lanes with fresh PASS evidence.
- Treat `PO-007`, `PO-008`, `PO-009`, `PO-011`, and `PO-012` as planned downstream evidence-policy gates with `waiver_policy` metadata, not proof passes and not contract-time waivers.
- Treat `PO-010` and `PO-019` as later state obligations; no State 5 pass is claimed.
