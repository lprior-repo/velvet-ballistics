reviewer_skill: proof-reviewer
reviewer_invocation_id: vb-b8i8f-state6-proof-reviewer-attempt3
proof_writer_invocation_id: vb-b8i8f-state5-proof-writer-attempt2
review_state: 6
reviewed_at: 2026-05-29T23:30:00Z

# Proof Review: vb-b8i8f — Cancel/Kill Lattice Storage Gap Recovery (RETRY attempt 3)

## Review Metadata

| Field | Value |
|-------|-------|
| Bead | vb-b8i8f |
| State | 6 (proof-reviewer RETRY attempt 3) |
| Reviewer invocation | vb-b8i8f-state6-proof-reviewer-attempt3 |
| Writer invocation | vb-b8i8f-state5-proof-writer-attempt2 |
| Previous review | vb-b8i8f-state6-proof-reviewer-attempt2 (seq 9, REJECTED, 10 findings: 6 CRITICAL, 2 HIGH, 2 LOW) |
| Independent, non-self-approved | YES — reviewer != writer; ledger seq 8 (writer attempt 2) → seq 10 (reviewer attempt 3) |
| Source checkout | /home/lewis/src/velvet-ballistics (control plane) |
| Workspace | /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-b8i8f |

## Controller Fix Assessment

The femdation controller claimed: "Added mod declarations for flux in codec/mod.rs. Kani file is in verification/kani/ directory. Flux file is in lifecycle/."

### Fix Verification

| Claim | Status | Evidence |
|-------|--------|----------|
| "added mod declarations for flux in codec/mod.rs" | ✅ APPLIED | `codec/mod.rs:96-97`: `#[cfg(feature = "flux")] pub mod flux_validation;` present in isolated workspace. |
| "Kani file is in verification/kani/ directory" | ⚠️ PRESENT, NOT WIRED | `verification/kani/kani_cancel_kill_lattice.rs` (380 lines, 20 harnesses) exists on disk but no `verification/mod.rs` or `verification.rs` declares it; lib.rs inline module only includes `proptest`. DEAD CODE. |
| "Flux file is in lifecycle/" | ⚠️ PRESENT, NOT WIRED | `lifecycle/flux_cancel_kill.rs` (194 lines) exists on disk but `lifecycle.rs` uses `include!()` for 3 chunks only; no `mod flux_cancel_kill;` declaration. DEAD CODE. |

### Controller Missed Fixes

The controller did NOT address:
1. Verus spec-to-production disconnection (GOD RULE 2 — same issue from attempts 1+2)
2. Kani boolean-model harness vacuity (GOD RULE 1+4 — 10/20 harnesses use local boolean variables)
3. Flux `#[trusted]` abuse on all model functions (including const fn `is_known_record_kind`)
4. Missing `flux_rs` dependency in both vb_runtime and vb_storage Cargo.toml files
5. Missing `flux` feature definition in both Cargo.toml `[features]` sections
6. Pre-existing `proptest_storage.rs:317` compile error blocking evidence collection

## Previous Attempt 2 Findings Resolution

| Finding ID | Attempt 2 | Status | Notes |
|------------|-----------|--------|-------|
| PF-VB-B8I8F-VERUS-DETACHED-R2-001 | Critical | NOT RESOLVED | Verus specs still disconnected from production exec fn |
| PF-VB-B8I8F-VERUS-DETACHED-R2-002 | Critical | NOT RESOLVED | Same — storage_kind_family.rs model-only |
| PF-VB-B8I8F-KANI-UNWIRED-R2-001 | Critical | NOT RESOLVED | Kani file still dead code |
| PF-VB-B8I8F-KANI-VACUOUS-R2-002 | Critical | NOT RESOLVED | 10 boolean-model harnesses unchanged |
| PF-VB-B8I8F-FLUX-UNWIRED-R2-001 | Critical | NOT RESOLVED | Flux lifecycle file still dead code |
| PF-VB-B8I8F-FLUX-UNWIRED-R2-002 | Critical | PARTIALLY RESOLVED | codec/mod.rs NOW has `mod flux_validation;` — but `flux_rs` dep missing, `flux` feature missing, all functions still `#[trusted]` |
| PF-VB-B8I8F-FLUX-TRUST-ABUSE-R2-002 | High | NOT RESOLVED | All Flux functions still `#[trusted]` (amplified — now wired but still unverifiable) |
| PF-VB-B8I8F-PROPTEST-PREEXISTING-R2-001 | High | NOT RESOLVED | proptest compile error unchanged |
| PF-VB-B8I8F-DEPRECATED-R2-001 | Low | NOT RESOLVED | `#[is_variant]` still present |
| PF-VB-B8I8F-NAMING-R2-001 | Low | NOT RESOLVED | 9 non_snake_case unchanged |

**Resolution score: 0.5 of 10 findings resolved (FLUX-UNWIRED-R2-002 partially addressed through mod declaration, but still inoperable).**

## Fresh Evidence Collection (Attempt 3)

### Module Wiring Check

```
=== vb_storage/src/codec/mod.rs ===
Line 96: #[cfg(feature = "flux")]
Line 97: pub mod flux_validation;
✅ flux_validation.rs is mod-declared (controller fix applied)

=== vb_runtime/src/shard/lifecycle.rs ===
Uses include!("lifecycle/chunk_001.rs"), chunk_002, chunk_003
NO mod flux_cancel_kill; declaration
❌ flux_cancel_kill.rs is dead code

=== vb_runtime/src/verification/ ===
Directory: kani/, proptest/
NO mod.rs file exists
❌ kani_cancel_kill_lattice.rs is dead code

=== vb_runtime/src/lib.rs ===
Line 92-98: #[cfg(test)] mod verification { pub(crate) mod proptest; }
NO #[cfg(kani)] pub mod kani; declaration
❌ kani harnesses unreachable
```

### Dependency Check

```
=== vb_storage/Cargo.toml ===
grep flux_rs: 0 matches
grep '[features]': exists but 'flux' feature NOT defined
❌ flux_validation.rs won't compile even if wired

=== vb_runtime/Cargo.toml ===
grep flux_rs: 0 matches
grep '[features]': exists but 'flux' feature NOT defined
❌ flux_cancel_kill.rs won't compile even if wired

=== vb_runtime depends on vb_storage, vb_core? ===
Cargo.toml lines 14-15: vb_core, vb_storage both listed
✅ Kani harness imports would resolve if file were wired
```

### Flux Notation Check

```
grep '#\[flux_rs::trusted\]' flux_validation.rs: 8+ matches
grep '#\[flux_rs::trusted\]' flux_cancel_kill.rs: 11 matches
grep '#\[flux_rs::sig\]' validation.rs: 0 matches
❌ All Flux annotations are #[trusted] — nothing is verified
```

### Verus Production Binding Check

```
grep 'requires\|ensures' chunk_002.rs: 0 matches
grep 'requires\|ensures' validation.rs: 0 matches
grep 'use vstd\|verus!' chunk_002.rs: 0 matches
❌ Zero Verus contracts on production functions
```

## Findings Summary (by severity)

### CRITICAL (6 — lethal, rejects)

1. **PF-VB-B8I8F-VERUS-DETACHED-R3-001**: GOD RULE 2 — Verus cancel_kill_lattice.rs model-only; 0 requires/ensures on production handle_cancel/handle_kill. Obligations: PO-VERUS-001..003.

2. **PF-VB-B8I8F-VERUS-DETACHED-R3-002**: GOD RULE 2 — Verus storage_kind_family.rs model-only; 0 requires/ensures on production is_known_record_kind/validate_kind_family. Obligations: PO-VERUS-004..005.

3. **PF-VB-B8I8F-KANI-UNWIRED-R3-001**: Kani runtime file dead code — no verification/mod.rs, no kani/mod.rs, no kani declaration in lib.rs. 20 harnesses compile nowhere. Obligations: PO-KANI-001..003.

4. **PF-VB-B8I8F-KANI-VACUOUS-R3-002**: GOD RULE 1+4 — 10 of 20 harnesses use local bool variables modeling IndexMap/IndexSet. Prove boolean arithmetic, not data structure semantics. Obligations: PO-KANI-001..003.

5. **PF-VB-B8I8F-FLUX-LIFECYCLE-UNWIRED-R3-001**: Flux lifecycle file dead code — not mod-declared in lifecycle.rs, flux_rs dependency missing. Obligations: PO-FLUX-001..003.

6. **PF-VB-B8I8F-FLUX-TRUST-ABUSE-R3-001**: GOD RULE 2 — All Flux functions #[trusted]; no #[sig] on production fns; const fn is_known_record_kind wrapped in trusted instead of verified; flux feature and dep missing even where mod declaration exists. Obligations: PO-FLUX-004..005.

### HIGH (2)

7. **PF-VB-B8I8F-FLUX-MISSING-DEP-R3-001**: flux_rs missing from vb_runtime Cargo.toml — NEW finding. Even if wired, files wouldn't compile.

8. **PF-VB-B8I8F-PROPTEST-BLOCKED-R3-001**: Pre-existing proptest_storage.rs:317 syntax error blocks evidence for PO-PROP-004/005.

### MEDIUM (1)

9. **PF-VB-B8I8F-CODE-DUPLICATION-R3-001**: NEW — 4 independent sources of truth for kind family definition (validation.rs, flux_validation.rs, kani_record_kind.rs, storage_kind_family.rs). Maintainability hazard.

### LOW (2)

10. **PF-VB-B8I8F-DEPRECATED-R3-001**: `#[is_variant]` deprecation warnings in cancel_kill_lattice.rs. Non-blocking.

11. **PF-VB-B8I8F-NAMING-R3-001**: 9 non_snake_case warnings in storage_kind_family.rs. Non-blocking.

## Contract Parity Assessment

| Contract Clause | Obligations | Evidence Quality |
|----------------|------------|-----------------|
| Cancel operates only on live runs | PO-VERUS-001, PO-KANI-001, PO-FLUX-001, PO-PROP-001 | VERUS: rejected (disconnected). KANI: rejected (dead code + 10/20 vacuous). FLUX: rejected (dead code + all trusted). PROPTEST: ✅ 10/10 pass |
| Kill operates only on live runs | Same cluster | Same assessment |
| Single terminal winner | PO-VERUS-002, PO-KANI-002, PO-FLUX-002, PO-PROP-002 | VERUS: rejected. KANI: rejected. FLUX: rejected. PROPTEST: ✅ pass |
| Stale authority rejected after terminal | PO-VERUS-003, PO-KANI-003, PO-FLUX-003, PO-PROP-003 | VERUS: rejected. KANI: rejected. FLUX: rejected. PROPTEST: ✅ pass |
| Kind 28 admitted to storage codec | PO-VERUS-004, PO-KANI-004, PO-FLUX-004, PO-PROP-004, PO-FUZZ-001 | VERUS: rejected (disconnected). KANI: ✅ wired in lib.rs, production-bound. FLUX: rejected (trusted+missing dep). PROPTEST: ⚠️ blocked by compile error. FUZZ: ⚠️ artifact exists, not executed |
| Replay ordinal contiguity preserved | PO-VERUS-005, PO-KANI-005, PO-FLUX-005, PO-PROP-005, PO-FUZZ-002 | VERUS: rejected (disconnected). KANI: ✅ wired. FLUX: rejected. PROPTEST: ⚠️ blocked. FUZZ: ⚠️ artifact exists, not executed |

## Obligation Evidence Matrix

| Obligation | Verifier | Can Run? | Non-Vacuous? | Bound to Production? | VERDICT |
|-----------|----------|----------|-------------|---------------------|---------|
| PO-VERUS-001 | Verus | ✅ compiles | ❌ model-only | ❌ no requires/ensures on exec fn | ❌ REJECTED |
| PO-VERUS-002 | Verus | ✅ compiles | ❌ model-only | ❌ no requires/ensures on exec fn | ❌ REJECTED |
| PO-VERUS-003 | Verus | ✅ compiles | ❌ model-only | ❌ no requires/ensures on exec fn | ❌ REJECTED |
| PO-VERUS-004 | Verus | ✅ compiles | ❌ model-only | ❌ no requires/ensures on exec fn | ❌ REJECTED |
| PO-VERUS-005 | Verus | ✅ compiles | ❌ model-only | ❌ no requires/ensures on exec fn | ❌ REJECTED |
| PO-KANI-001 | Kani | ❌ dead code | ❌ 5/10 boolean models | ⚠️ 5/10 production types | ❌ REJECTED |
| PO-KANI-002 | Kani | ❌ dead code | ❌ 5/5 boolean models | ❌ no IndexMap/IndexSet instances | ❌ REJECTED |
| PO-KANI-003 | Kani | ❌ dead code | ❌ 5/5 boolean models | ❌ no IndexMap/IndexSet instances | ❌ REJECTED |
| PO-KANI-004 | Kani | ✅ wired (lib.rs:44) | ✅ exhaustive kind-space | ✅ uses production validation.rs | ✅ PASSING |
| PO-KANI-005 | Kani | ✅ wired (lib.rs:44) | ✅ replay sequence checks | ✅ uses production codec types | ✅ PASSING |
| PO-FLUX-001 | Flux | ❌ dead code + missing dep | ❌ all #[trusted] | ❌ no #[sig] on production fn | ❌ REJECTED |
| PO-FLUX-002 | Flux | ❌ dead code + missing dep | ❌ all #[trusted] | ❌ no #[sig] on production fn | ❌ REJECTED |
| PO-FLUX-003 | Flux | ❌ dead code + missing dep | ❌ all #[trusted] | ❌ no #[sig] on production fn | ❌ REJECTED |
| PO-FLUX-004 | Flux | ❌ missing dep + no feature | ❌ all #[trusted] | ❌ no #[sig] on production fn | ❌ REJECTED |
| PO-FLUX-005 | Flux | ❌ missing dep + no feature | ❌ all #[trusted] | ❌ no #[sig] on production fn | ❌ REJECTED |
| PO-PROP-001 | Proptest | ✅ 10/10 pass | ✅ real assertions | ✅ exercises production API | ✅ PASSING |
| PO-PROP-002 | Proptest | ✅ 10/10 pass | ✅ real assertions | ✅ exercises production API | ✅ PASSING |
| PO-PROP-003 | Proptest | ✅ 10/10 pass | ✅ real assertions | ✅ exercises production API | ✅ PASSING |
| PO-PROP-004 | Proptest | ❌ compile error blocks | N/A | N/A | ⚠️ BLOCKED |
| PO-PROP-005 | Proptest | ❌ compile error blocks | N/A | N/A | ⚠️ BLOCKED |
| PO-FUZZ-001 | Fuzz | ⚠️ not executed | N/A | N/A | ⚠️ PENDING |
| PO-FUZZ-002 | Fuzz | ⚠️ not executed | N/A | N/A | ⚠️ PENDING |

**Summary: 4/22 obligations have genuine evidence (PO-PROP-001..003, PO-KANI-004..005). 14/22 rejected for fatal gaps. 4/22 pending or blocked.**

## Non-Vacuity Assessment

- **Verus**: Non-vacuous for the spec model (18 lemmas proven about model internals). VACUOUS for the implementation — zero contracts on production exec fn. The `classify_run_has_correct_semantics` trusted bridge returns `true` unconditionally — this is a vacuous trust anchor. **Rejected.**
- **Kani record_kind (PO-KANI-004..005)**: GOD RULE 1 compliant; wired into vb_storage via lib.rs:44; exercises production validation.rs functions with kani::any() inputs. Non-vacuous. **Passing.**
- **Kani runtime (PO-KANI-001..003)**: 10 harnesses genuinely exercise production types (JournalEvent::RunKilled, is_known_record_kind, validate_kind_family). 10 harnesses model IndexMap/IndexSet with local boolean variables. File not wired into crate. **Rejected.**
- **Flux**: Zero refinement checking occurs because (a) files are dead code (lifecycle) or (b) missing flux_rs dependency (codec) or (c) all functions are #[trusted]. Even where codec/mod.rs now declares the module (controller fix), the module cannot compile and would trust-assume everything. **Rejected.**
- **Proptest**: PO-PROP-001..003: 10/10 pass with real assertions against production API. Non-vacuous. PO-PROP-004..005: blocked by pre-existing compile error. **Partially passing.**
- **Fuzz**: Artifacts exist but not executed. PENDING_FORMAL_EXECUTION. **Pending.**

## Trusted Base Assessment

- TBR-001 through TBR-015 unchanged from previous reviews.
- TBR-014 (BLOCK-001): ✅ RESOLVED — validation range extended to 10..=28.
- TBR-015 (BLOCK-002): STILL BLOCKED — full Shard Kani construction requires SharedRuntimeJournal → Fjall chain.
- TBR-FLUX-001 through TBR-FLUX-004: Flux trusted boundaries declared but inoperable (files unwired, missing dependency, all functions #[trusted]).
- TBR-VERUS-001: `#[verifier::external_body]` on `classify_run_has_correct_semantics` returns `true` — vacuous trust anchor.
- No new unledgered trust markers detected.

## Blocker Analysis

| Blocker | Severity | Status |
|---------|----------|--------|
| vb-b8i8f-BLOCK-001 | RESOLVED | validation.rs:24,46 extended to 10..=28 ✅ |
| vb-b8i8f-BLOCK-002 | STILL BLOCKED | Full Shard Kani construction requires SharedRuntimeJournal → Fjall dependency chain |

## Verdict

**STATUS: REJECTED**

The controller applied exactly one fix (codec/mod.rs `mod flux_validation;` declaration), fixing 0.5 of 10 previous findings. **Nine lethal/high gaps remain across Verus, Kani, and Flux verification artifacts:**

1. **5 Verus obligations** (PO-VERUS-001..005): GOD RULE 2 still violated — spec proves model-internal lemmas but has zero `requires`/`ensures` on production exec fn. The Verus specs are correct mathematical models, not implementation proofs. **Same core issue for 3 consecutive reviews.**
2. **3 Kani runtime obligations** (PO-KANI-001..003): Dead code (unwired) + 10/20 harnesses structurally vacuous. Even the 10 genuinely production-exercising harnesses produce zero evidence.
3. **5 Flux obligations** (PO-FLUX-001..005): Dead code (lifecycle) or inoperable (codec: missing dependency, missing feature, all trusted). Everything is `#[trusted]` — nothing is verified.

**Only 4 of 22 obligations have genuine evidence**: PO-PROP-001..003 (proptest, 10/10 passing) and PO-KANI-004..005 (existing Kani harnesses in vb_storage, wired and production-bound). These 4 obligations cover the proptest behavioral properties and the storage kind-family verification. They prove the RunKilled(28) admission works at the property-test and bounded-verification levels.

The Verus, Kani-runtime, and Flux lanes — representing the formal verification tier — have produced zero implementation evidence. The Verus spec models are mathematically sound (18+18 lemmas proven) but unattached to production code. The Kani runtime harnesses exist but are invisible to the verifier. The Flux refinement files exist but can neither compile nor verify.

**Eleven findings (6 CRITICAL, 2 HIGH, 1 MEDIUM, 2 LOW) prevent advancement to State 7.**

## Repair Guide

### Before Resubmit (lethal gaps):

1. **Verus production binding**: Add `#[verus::spec]` or `requires`/`ensures` to `Shard::handle_cancel`, `Shard::handle_kill` in chunk_002.rs, and to `is_known_record_kind`, `validate_kind_family` in validation.rs. These must reference the existing spec models (spec_terminalize, spec_is_known_record_kind, spec_validate_kind_family). The spec models are correct — they just need to be attached to exec fn.

2. **Kani wiring**: Create `verification/mod.rs` with `#[cfg(kani)] pub mod kani;` AND `verification/kani/mod.rs` with `mod kani_cancel_kill_lattice;`. Remove or rework the 10 boolean-model harnesses (lines 178-343) — replace with actual IndexMap/IndexSet allocations or reduce to just the 10 production-exercising harnesses (lines 39-166, 364-379).

3. **Flux dependency + feature**: Add `flux_rs = { version = "0.1", optional = true }` to both vb_runtime and vb_storage Cargo.toml, and define `[features] flux = ["flux_rs"]` in each. Wired module declarations already reference `#[cfg(feature = "flux")]`.

4. **Flux lifecycle wiring**: Add `#[cfg(feature = "flux")] mod flux_cancel_kill;` to lifecycle.rs.

5. **Flux trusted reduction**: Remove `#[trusted]` from `model_is_known_record_kind` (const fn, Flux can verify directly). Apply `#[flux_rs::sig]` directly to production `is_known_record_kind` in validation.rs. Keep `#[trusted]` on `validate_kind_family` model with explicit compensating evidence refs.

### After Resubmit (non-blocking):

6. **Proptest compile fix**: Fix proptest_storage.rs:317 to unblock evidence for PO-PROP-004/005.

7. **Verus deprecation**: Replace `#[is_variant]` with `->` or `matches` pattern.

8. **Verus naming**: Fix 9 non_snake_case function names or add `#[allow(non_snake_case)]`.

9. **Fuzz execution**: Run `cargo +nightly fuzz run kind_validation` and `journal_decode` and capture output as evidence.
