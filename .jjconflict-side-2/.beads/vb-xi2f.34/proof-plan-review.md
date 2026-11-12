# Proof Plan Review — vb-xi2f.34: Finish Digest Coverage

**Reviewer Skill**: proof-plan-reviewer
**Reviewer Invocation ID**: proof-plan-reviewer-vb-xi2f.34-20260524
**Review State**: p4-review
**Date**: 2026-05-24

---

## Review Summary

This is a P1 (proportional) review of the proof plan for bead vb-xi2f.34: "P1: digest covers finish semantics." The plan covers ~22 lines of Rust in `digest_step_primitive` (Finish arm, lines 150-156 canonical + 250-255 legacy) and `canonical_digest` (lines 116-138).

**Result**: APPROVED with 5 findings (0 CRITICAL, 1 MEDIUM, 3 LOW, 1 INFO). All findings are documentation/precision improvements; none block proof writing.

---

## Reviewed Artifacts

| Artifact | Hash/Version | Status |
|---|---|---|
| `contract.md` | rust-contract, 10 clauses (C1-C10) | Reviewed |
| `domain-model.md` | rust-contract, 7 invariants (INV-1 to INV-7) | Reviewed |
| `type-contracts.md` | rust-contract, 4 type contracts + duplicate code analysis | Reviewed |
| `workflow-model.md` | rust-contract, 5 states, 3 transitions, 4 temporal hazards | Reviewed |
| `error-taxonomy.md` | rust-contract, 3-layer error classification | Reviewed |
| `boundary-map.md` | rust-contract, pure-core/imperative-shell split | Reviewed |
| `hazard-analysis.md` | rust-contract, 9 hazards (HAZ-1 to HAZ-9) | Reviewed |
| `proof-seeds.jsonl` | 10 seeds (PS-FINISH-DIGEST-001 through PS-FINISH-DIGEST-010) | Reviewed |
| `traceability-matrix.jsonl` | 10 matrix rows (TR-MATRIX-001 through TR-MATRIX-010) | Reviewed |
| `proof-strategy.md` | 4-layer defense-in-depth strategy | Reviewed |
| `verifier-lane-decisions.jsonl` | 19 lane decisions (13 required, 6 not_applicable) | Reviewed |
| `proof-obligations.planned.jsonl` | 13 obligations (3 Kani, 4 proptest, 4 integration, 2 static) | Reviewed |
| `trusted-base-plan.md` | 4 trusted deps, 2 stubs, 3 bounds, 3 trusted surfaces | Reviewed |
| `proof-to-implementation-input.md` | Bridge: source mapping, file plan, visibility, mock design | Reviewed |
| `proof-coverage-matrix.md` | Coverage by clause, seed, hazard, gap | Reviewed |
| `waiver-candidates.jsonl` | 3 waiver candidates (WC-001 through WC-003) | Reviewed |
| `agent-invocation-ledger.jsonl` | 1 entry (femdation setup) | Reviewed |
| `delivery-scope.jsonl` | Crate/test/gap/risk audit | Reviewed |
| `baseline-report.md` | Workspace baseline | Reviewed |

---

## Core Verifier Lane Coverage

| Verifier | Decision | Obligations | Reviewer Assessment |
|---|---|---|---|
| **Kani** | REQUIRED | 3 (PO-KANI-001 through 003) | ACCEPTED — well-scoped bounded proofs with tracking mock |
| **Proptest** | REQUIRED | 4 (PO-PROPTEST-001 through 004) | ACCEPTED — statistical defense-in-depth across large input spaces |
| **Integration Test** | REQUIRED | 4 (PO-INT-001 through 004) | ACCEPTED — end-to-end pipeline validation |
| **Static Analysis** | REQUIRED | 2 (PO-STATIC-001, 002) | ACCEPTED — exhaustiveness + unsafe/IO audit |
| **TLA+** | NOT_APPLICABLE | — | ACCEPTED — pure function, no interleavings |
| **Verus** | NOT_APPLICABLE | — | ACCEPTED — behavioral properties, external blake3 dep |
| **Flux** | NOT_APPLICABLE | — | ACCEPTED — no data refinements needed |
| **Loom** | NOT_APPLICABLE | — | ACCEPTED — zero concurrency |
| **Miri** | NOT_APPLICABLE | — | ACCEPTED — #![forbid(unsafe_code)] |
| **cargo-fuzz** | NOT_APPLICABLE | — | ACCEPTED — typed AST not raw bytes |

All 19 lane decisions have corresponding `verifier-lane-review/v1` rows in `verifier-lane-review.jsonl`.

---

## Obligation Completeness

All 13 obligations include: `schema_version: proof-obligation/v1`, exact command, workdir, bounds, assumptions, and expected evidence. No legacy alias fields detected.

### Obligation-by-Obligation Assessment

| Obligation | Schema | Command | Workdir | Bounds | Assumptions | Evidence | Status |
|---|---|---|---|---|---|---|---|
| PO-KANI-FINISH-001 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-KANI-FINISH-002 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-KANI-FINISH-003 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-PROPTEST-FINISH-001 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-PROPTEST-FINISH-002 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-PROPTEST-FINISH-003 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-PROPTEST-FINISH-004 | v1 | Exact | Set | **Vague** ⚠ | Documented | Specific | PASS* |
| PO-INT-FINISH-001 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-INT-FINISH-002 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-INT-FINISH-003 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-INT-FINISH-004 | v1 | Exact | Set | Clear | Documented | Specific | PASS |
| PO-STATIC-FINISH-001 | v1 | Exact | Set | **Imprecise** ⚠ | Documented | Specific | PASS* |
| PO-STATIC-FINISH-002 | v1 | Exact | Set | Clear | Documented | Specific | PASS |

*See findings PPF-FINISH-001 and PPF-FINISH-003 for documentation improvement recommendations.

---

## Defense-in-Depth Layering

```
LAYER 1: Kani Bounded Proofs ──────────────────── injectivity, discrimination
         ↓ (3 harnesses, bounded input spaces)
LAYER 2: Proptest Statistical ─────────────────── determinism, sensitivity
         ↓ (4 properties, 10,000+ trials each)
LAYER 3: Integration Tests ────────────────────── end-to-end pipeline
         ↓ (4 tests, concrete YAML fixtures)
LAYER 4: Static Analysis ──────────────────────── exhaustiveness, unsafe audit
         ↓ (2 checks, grep + compile-time assertions)
```

Each layer provides independent evidence. Layer 1 proves formal properties on bounded domains. Layer 2 extends coverage to larger/unbounded domains statistically. Layer 3 validates the full compilation pipeline. Layer 4 enforces structural correctness. This is a well-designed defense-in-depth strategy for P1 scope.

---

## Non-Vacuity Assessment

The Kani harnesses use a **tracking mock** for `blake3::Hasher` rather than real blake3. The mock records byte sequences fed to `hasher.update()` and the proof claims are about input discrimination (distinct inputs → distinct byte sequences), not hash correctness. This is a valid model reduction documented in trusted-base-plan.md (S-1) because:

1. blake3 is trusted as collision-resistant (T-1).
2. If byte sequences differ, the final hashes differ with overwhelming probability (~1 - 2^-128).
3. Proptest defense-in-depth (Layer 2) exercises the real blake3 path, catching any mock/reality divergence.

**Non-vacuity risk**: Low. The mock correctly models the property of interest (input discrimination). Finding PPF-FINISH-004 documents the requirement that the mock must faithfully track update() calls in order.

---

## Waiver Assessment

| Waiver | Clause | Behavior-Affecting | Compensating Evidence | Status |
|---|---|---|---|---|
| WC-001 | HAZ-9: canonical_primitive_name bugs | **No** (Finish bypasses) | Kani canonical_name harness | ACCEPTED |
| WC-002 | C8: _ arm produces "unsupported" | **Yes** (acknowledged gap) | PO-STATIC-FINISH-001 exhaustiveness test | ACCEPTED* |
| WC-003 | C7: Legacy path duplicate code | **No** (equivalence test) | PO-INT-FINISH-004 equivalence test | ACCEPTED |

*WC-002 waives a behavior-affecting property but (a) the current 2-variant enum makes the _ arm unreachable, (b) the contract clause C8 explicitly permits this behavior, and (c) PO-STATIC-FINISH-001 provides ongoing test coverage. Finding PPF-FINISH-005 documents this acceptance.

---

## Trusted Base Assessment

The trusted-base-plan.md identifies 4 trusted dependencies, 2 stubs, and 3 model reductions:

| ID | Component | Risk | Mitigation | Adequate? |
|---|---|---|---|---|
| T-1 | blake3 determinism/collision-resistance | Medium | Industry-standard crate; integration test catch | Yes |
| T-2 | i64::to_le_bytes() bijection | None | Rust stdlib guarantee | Yes |
| T-3 | String::as_bytes() determinism | None | Rust stdlib guarantee | Yes |
| T-4 | #[non_exhaustive] semantics | None | Rust language guarantee | Yes |
| S-1 | Kani blake3 tracking mock | Low | Proptest defense-in-depth | Yes |
| S-2 | Proptest WorkflowSource generator | Low | Kani exhaustive bounded exploration | Yes |
| B-1 | String ≤ 256 bytes (Kani) | Low | Proptest catches >256B edge cases | Yes |

All trusted components are appropriate for P1 scope. No untracked assumptions identified.

---

## Bridge Planning

`proof-to-implementation-input.md` provides:
- Exact source file and line number mappings for all 13 obligations
- File creation plan for 4 new test/harness files
- Visibility requirements (canonical path: `pub(super)` OK; legacy path: may need `#[cfg(test)]` re-export)
- Kani mock design with pseudocode
- Implementation order (static → integration → proptest → Kani)
- Expected evidence commands (verbatim copy from obligations)

The bridge is sufficient for proof-writer and proof-to-implementation agents.

---

## Findings Summary

| ID | Severity | Code | Obligation | Description |
|---|---|---|---|---|
| PPF-FINISH-001 | MEDIUM | E_OBLIGATION_BOUNDS_VAGUE | PO-PROPTEST-FINISH-004 | Bounds field describes structural argument, not concrete test inputs |
| PPF-FINISH-002 | LOW | E_PROOF_PLAN_PROVENANCE | N/A | Missing distinct proof-planner invocation in agent-invocation-ledger |
| PPF-FINISH-003 | LOW | E_OBLIGATION_BOUNDS_VAGUE | PO-STATIC-FINISH-001 | Bounds conflates compile-time intent with runtime test mechanic |
| PPF-FINISH-004 | INFO | E_KANI_MOCK_ATTESTATION | PO-KANI-001,002,003 | Kani mock must faithfully track update() calls in order |
| PPF-FINISH-005 | LOW | E_WAIVER_BEHAVIOR_AFFECTING | WC-002 | Behavior-affecting waiver accepted with compensating test |

Detailed findings in `proof-plan-findings.jsonl`.

---

## Contract Clause Coverage

All 10 contract clauses (C1-C10) are covered by at least one obligation:

| Clause | Primary Coverage | Defense-in-Depth |
|---|---|---|
| C1: Result sensitivity | Kani (PO-001, 002) | Proptest (PO-002), Integration (PO-001) |
| C2: Step ID sensitivity | Integration (PO-002) | Proptest (via C3) |
| C3: Position sensitivity | Proptest (PO-003) | — |
| C4: Determinism | Proptest (PO-001) | — |
| C5: Variant discrimination | Kani (PO-003) | Integration (PO-003) |
| C6: Digest survives compile | Integration (PO-001) | — |
| C7: Single implementation | Integration (PO-004) | — |
| C8: Forward compatibility | Static (PO-001) | Waiver WC-002 |
| C9: Pre-validation scope | Proptest (PO-004) | Structural guarantee |
| C10: Runtime exclusion | Static (PO-002) | Code review |

No uncovered contract clauses. All 8 behavior-affecting clauses have at least one verifier lane assigned.

---

## Source Code Verification

Source code audit confirms:
- `canonical_digest()` exists at `part_05.rs:116-138` ✓
- `digest_step_primitive()` exists at `part_05.rs:140-165` ✓
- Finish arm at lines 150-156 writes `b"finish"` prefix then encodes `ScalarValue` ✓
- `#![forbid(unsafe_code)]` present in `vb_compile/src/compile/mod.rs` and `vb_core/src/workflow/mod.rs` ✓
- `_` arm at line 155 produces `b"unsupported"` for unknown ScalarValue variants ✓
- `String` arm uses `value.as_bytes()` ✓
- `Integer` arm uses `value.to_le_bytes()` ✓

Source code matches proof plan claims.

---

## GOD RULE Compliance

| Rule | Status |
|---|---|
| #1: No hardcoded Kani shapes | ✅ Admonished in proof-to-implementation-input.md: "Use kani::any()" |
| #2: No vacuum Verus proofs | ✅ Verus NOT_APPLICABLE (no Verus proofs planned) |
| #3: No unbounded TLA+ math | ✅ TLA+ NOT_APPLICABLE (no TLA+ proofs planned) |
| #4: No loop oscillations | ✅ Not yet applicable (no proofs attempted) |
| #5: No blind verification mutations | ✅ Scope trimmed to digest function only |

---

## Proportionality Assessment (P1)

This is a P1 review for a ~22-line function. The proof plan is proportionate:
- **13 obligations** for 10 contract clauses and 8 gaps
- **4 verification layers** (formal → statistical → integration → structural)
- **No Verus or TLA+** (correctly excluded as not applicable)
- **3 Kani harnesses** for the core formal claims (injectivity, discrimination)
- **Trusted base** with 4 dependencies, 2 stubs — minimal and documented

The plan does not over-engineer. Each obligation closes a specific gap or validates a specific contract clause. No obligation exists without a corresponding proof seed and contract clause.

---

## Decision

**STATUS: APPROVED**

The proof plan is precise, complete, and proportionate for P1 scope. All 19 verifier lanes have accepted review rows. All 13 obligations have exact commands and documented bounds. The defense-in-depth layering is sound. The trusted base is documented. The bridge to implementation is prepared.

5 non-blocking findings are documented in `proof-plan-findings.jsonl`. No repair guide is needed.

---

## Next Steps

1. **Proof-writer** (go-skill state 5): Implement Kani harnesses, proptest properties, integration tests, and static checks per `proof-obligations.planned.jsonl` and `proof-to-implementation-input.md`.
2. **Proof-reviewer** (go-skill state 6): Review executed proof artifacts against this approved plan.
3. **Proof-to-implementation** (go-skill state 7): Bridge approved proof claims to Rust implementation obligations.
