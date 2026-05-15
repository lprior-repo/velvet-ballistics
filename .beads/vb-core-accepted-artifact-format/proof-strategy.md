# Proof Strategy — vb-core-accepted-artifact-format

## Bead & Workspace
- **Bead ID**: `vb-core-accepted-artifact-format`
- **Workspace**: `/tmp/vb-ws/vb-core-accepted-artifact-format`
- **Current State**: 4 (Proof Planning)

---

## Critical-First Gate: KANI-MISMATCH-001

The **gate_count mismatch** is the central defect. All other obligations are scoped relative to it.

| Obligation | Lane | Finding |
|------------|------|---------|
| `KANI-MISMATCH-001` | kani | **CRITICAL**: `submit_artifact` produces `gate_count=2`; `load_accepted_artifact` under Strict requires 15 → `InvalidGateCount { found: 2, required: 15 }` |

**Lane ordering is sequenced so KANI-MISMATCH-001 runs FIRST** as a blocking gate. If this obligation fails with a counterexample confirming the mismatch, all downstream proof gates are deferred until resolution (Option A/B/C/D from contract.md §Resolution Options).

---

## Verifier Lane Map

### Lane 1 — Kani (Blocking / Critical-First)

| Obligation | Harness | Expected Evidence |
|------------|---------|-------------------|
| `KANI-GATE-001` | `submit_artifact_harness` | 0 counterexamples for `gate_count ∈ 0..16` |
| `KANI-MISMATCH-001` | `gate_count_mismatch_harness` | **Counterexample confirming `InvalidGateCount { found: 2, required: 15 }`** — this IS the expected result |

**KANI-MISMATCH-001 expected outcome**: The harness must construct a scenario where `submit_artifact` is called, then `load_accepted_artifact` is called under Strict policy. Kani is expected to find a counterexample showing the artifact with `gate_count=2` is rejected. This is NOT a spurious finding — it is the known mismatch being formally verified.

**Harness specification for `gate_count_mismatch_harness`**:
```
1. Construct CompiledWorkflow via CompiledWorkflow::try_from_parts(...)
2. Call submit_artifact(journal, &workflow, Relaxed) → Ok(artifact)
3. Call load_accepted_artifact(artifact_store, &artifact.digest, Strict)
4. Assert: result is Err(InvalidGateCount { found: 2, required: 15 })
```

Kani must verify this counterexample exists. The negative result (no counterexample) would mean the mismatch is somehow resolved — which would itself be an anomaly.

### Lane 2 — TLA+ (Protocol / Two Specs)

| Obligation | Spec | Invariants | Expected Evidence |
|------------|------|------------|-------------------|
| `TLA-ARTIFACT-001` | `ArtifactAdmission.tla` + `.cfg` | `ArtifactAdmittedImpliesValidGateCount`, `StrictPolicyRejectsTwoGate` | TLC: 0 invariant violations in 20-step BMC |
| `TLA-ARTIFACT-002` | `ArtifactDigest.tla` + `.cfg` | `DigestMatchesIR` | TLC: 0 invariant violations |

**Run order**: `TLA-ARTIFACT-001` first (covers the mismatch scenario at protocol level); `TLA-ARTIFACT-002` second (digest invariant).

**Spec locations**: Both specs must exist at `specs/ArtifactAdmission.tla` and `specs/ArtifactDigest.tla` in the workspace.

### Lane 3 — Verus (Four Obligations)

| Obligation | Target | Spec/Proof Fn | Mode |
|------------|--------|---------------|------|
| `VERUS-INV-001` | `AcceptedArtifact` | `accepted_artifact_digest_matches_ir` | verify-proof |
| `VERUS-INV-002` | `VerificationProof` | `gate_count_valid` | verify-proof |
| `VERUS-INV-003` | `VerificationProof::new` | `proof_flags_not_hardcoded` | **verify-standard** (documents hardcoded flags as violation) |
| `VERUS-PRE-001` | `CompiledWorkflow::try_from_parts` | `proof_try_from_parts_sole_constructor` | verify-proof |

**Execution constraint**: VERUS-INV-003 is `verify-standard` — it is expected to **flag** the current hardcoded `true` values for proof flags as a known invariant violation. This is not a proof failure; it documents the gap until 15-gate implementation lands.

### Lane 4 — Miri (Two Obligations)

| Obligation | Test Target | Expected Evidence |
|------------|-------------|-------------------|
| `MIRI-DECODE-001` | `vb_runtime --test accepted_artifact_miri_decode` | 0 UB, 0 panics, 0 leaks |
| `MIRI-SAFETY-001` | `vb_storage --test accepted_artifact_miri` | 0 UB on `AcceptedArtifact` decode |

**Execution**: Both can run in parallel. Miri is slow; run on a dedicated thread.

### Lane 5 — Loom (Optional / Deferred)

| Obligation | Target | Mode |
|------------|--------|------|
| `LOOM-CONCURRENT-001` | `vb_runtime --test concurrent_artifact_store` | verify-deep (optional; `required: false`) |

Run only after all other lanes pass. If KANI-MISMATCH-001 fails and triggers BLOCK_LOCAL, defer Loom.

### Lane 6 — API Compatibility (Optional)

| Obligation | Target | Mode |
|------------|--------|------|
| `API-COMPAT-001` | `vb_storage AcceptedArtifact` pub fields | verify-standard |
| `API-COMPAT-002` | `vb_storage VerificationProof` pub fields | verify-standard |

Both `required: false`. Run in parallel after all critical lanes complete.

### Lane 7 — Fuzz (Deferred to Owner State 6)

| Obligation | Target | Mode |
|------------|--------|------|
| `FUZZ-DECODE-001` | `cargo fuzz run decode_accepted_artifact` | verify-deep |

`owner_state: 6` — not in scope for S4 proof planning. Planned for S6 execution.

---

## Dependency Graph

```
[KANI-MISMATCH-001] ──(counterexample found)──► BLOCK_LOCAL → defer all lanes
        │
        ├──(no counterexample)──► proceed
        │
[KANI-GATE-001] ──(0 counterexamples)──► pass
        │
[TLA-ARTIFACT-001] ──(0 violations)──► pass ──► [TLA-ARTIFACT-002] ──(0 violations)──► pass
        │
[VERUS-INV-001] ──(0 errors)──► pass
[VERUS-INV-002] ──(0 errors)──► pass
[VERUS-INV-003] ──(flag hardcoded flags)──► KNOWN_GAP ──► pass with documentation
[VERUS-PRE-001] ──(0 errors)──► pass
        │
[MIRI-DECODE-001] ──(0 UB)──► pass
[MIRI-SAFETY-001] ──(0 UB)──► pass
        │
[LOOM-CONCURRENT-001] ──(0 races)──► pass (optional)
[API-COMPAT-001] ──(0 violations)──► pass (optional)
[API-COMPAT-002] ──(0 violations)──► pass (optional)
```

---

## Gate Blocking Rules

| Block Type | Trigger | Action |
|------------|---------|--------|
| `BLOCK_LOCAL` | KANI-MISMATCH-001 finds counterexample confirming mismatch | All lanes blocked; escalate to resolution options |
| `BLOCK_REGRESSION` | Any VERUS/TLA+ invariant violation | Halt; fix before continuing |
| `REQUIRED_OBLIGATION_FAIL` | Any `required: true` obligation fails | Full bead blocked |
| `REQUIRED_OBLIGATION_FAIL` (optional) | Any `required: false` obligation fails | Document and continue |

---

## Expected Evidence Artifacts

| Artifact | Obligation | Type |
|----------|-----------|------|
| `kani-report.md` | KANI-GATE-001 | Kani HTML/JSON report summary |
| `kani-mismatch-report.md` | KANI-MISMATCH-001 | Kani counterexample trace |
| `tla-report.md` | TLA-ARTIFACT-001 | TLC stdout/stderr with 0 violations |
| `tla-digest-report.md` | TLA-ARTIFACT-002 | TLC stdout/stderr with 0 violations |
| `verus-report.md` | VERUS-INV-001/002/003, VERUS-PRE-001 | Verus output log |
| `miri-report.md` | MIRI-DECODE-001 | Miri summary |
| `miri-safety-report.md` | MIRI-SAFETY-001 | Miri summary |
| `loom-report.md` | LOOM-CONCURRENT-001 | Loom output |
| `semver-report.md` | API-COMPAT-001/002 | semver-checks output |
| `fuzz-report.md` | FUZZ-DECODE-001 | Fuzz summary (deferred to S6) |

---

## Resolution Options Status

Until KANI-MISMATCH-001 is formally resolved via one of the four options:

- **Option A** (ADMISSION_GATE_COUNT → 15): Full 15-gate verification in vb_storage
- **Option B** (REQUIRED_GATE_COUNT → 2): Relax policy to 2 gates
- **Option C** (retire 2-gate path): Deprecate and remove 2-gate path
- **Option D** (versioned format): Support both formats via version field

**All four options are code changes** — this bead's proof plan formally verifies the mismatch exists (KANI-MISMATCH-001) and that no bypass exists for INV-002 (gate_count bounds). The resolution itself belongs to a follow-on bead.
