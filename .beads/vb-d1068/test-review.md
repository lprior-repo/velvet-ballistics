# vb-d1068 Test Review

Reviewer: LOCAL3090 (independent)
Beacon: vb-3v011
Commit range: be65d9191527..a36705032ef5
Date: 2026-10-21

## Scope

Two test chunks in `crates/velvet_ballistics/tests/admission_evidence_integration/`:
- `chunk_001.rs` (3 test modifications)
- `chunk_002.rs` (1 test modification + 1 new test)

## Verdict: PASS

The test changes correctly replace forged-digest workflows with genuinely compiled
workflows where appropriate, and add a regression test asserting forged-digest rejection
across both Relaxed and Strict policies. All 149 regression tests pass.

## Findings

### Test 1: restart_lookup_finds_persisted_header (chunk_001.rs)

**Before:** Constructed `set_const_finish_workflow(digest)` with forged `[0x42; 32]` digest.
**After:** Uses `compile_set_const_finish("restart_lookup")` → genuine compiled workflow.

The test intent is persistence/lookup across journal reopen, not digest forgery. The
genuine compiler output ensures the canonical digest matches the serialized IR, so
`submit_artifact` succeeds under the test's policy. The test still validates that the
workflow header persists and is findable after journal reopen. ✓

**Finding 1 (correctness):** Test now exercises genuine artifact submission. Still
validates persistence/lookup semantics. ✓

### Test 2: journal_persistence_survives_runtime_drop_and_reopen (chunk_001.rs)

**Before:** Constructed `set_const_finish_workflow(digest)` with forged `[0x47; 32]` digest.
**After:** Uses `compile_set_const_finish("journal_persistence")` → genuine compiled workflow.

Same reasoning as Test 1. The test validates journal persistence across runtime drop/reopen.
Genuine artifact ensures submission succeeds, allowing the persistence semantics to be
tested. ✓

**Finding 2 (correctness):** Test now exercises genuine artifact submission. Still
validates journal persistence semantics. ✓

### Test 3: run_without_artifact_under_relaxed_policy (chunk_002.rs)

**Before:** Constructed `set_const_finish_workflow(digest)` with forged `[2; 32]` digest.
**After:** Uses `compile_set_const_finish("relaxed_no_artifact")` → genuine compiled workflow.

**Finding 3 (correctness):** The test comment was updated to reflect the actual contract:
"Relaxed admission skips artifact loading, but submit_artifact still checksum-validates
the canonical IR, so a genuine digest is required." This is accurate — `prepare_compiled_artifact`
always runs `checked_parts` which recomputes and validates the canonical-IR digest,
regardless of policy. The policy only affects persistence behavior (Strict does
`persist_strict()`). ✓

### Test 4: forged_digest_workflow_rejects_across_relaxed_and_strict (NEW, chunk_002.rs)

**Intent:** Regression test asserting a forged-digest workflow is rejected by
`submit_artifact` under BOTH `Relaxed` and `Strict`.

**Implementation:**
- Constructs `set_const_finish_workflow(forged_digest)` with `[0xE1; 32]` (placeholder digest ≠ canonical IR digest)
- Submits under `Relaxed` → expects `JournalError::ArtifactChecksumMismatch`
- Submits under `Strict` → expects `JournalError::ArtifactChecksumMismatch`

**Finding 4 (correctness):** This test correctly captures the typed evidence contract:
checksum validation is policy-independent. The legacy "relaxed skips checksum" contract
is superseded by schema-v3 typed evidence. The test uses the existing
`set_const_finish_workflow` helper (which forges a digest) — this is the appropriate
use case for that helper now that the other tests use genuine compilation. ✓

**Finding 5 (coverage):** The test covers both policy extremes (Relaxed and Strict).
Journaled would also reject, but Relaxed is the weaker policy; if Relaxed rejects,
Journaled/Strict must also reject. Testing both endpoints is sufficient. ✓

### compile_set_const_finish Helper (chunk_001.rs)

```rust
fn compile_set_const_finish(name: &str) -> Option<CompiledWorkflow> {
    let yaml = format!(
        "version: velvet-ballistics/v1\nname: {name}\nwhen:\n  manual: {{}}\nsteps:\n  - id: build\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n"
    );
    vb_compile::compile_workflow(yaml.as_bytes()).ok()
}
```

**Finding 6 (correctness):** Helper compiles a minimal valid workflow through the real
compiler. The YAML is well-formed with a valid version string, manual trigger, and two
steps (set-const then finish). Distinct from `set_const_finish_workflow` which forges a
placeholder digest. ✓

**Finding 7 (naming):** Clear distinction between `set_const_finish_workflow` (forged,
for rejection tests) and `compile_set_const_finish` (genuine, for happy-path tests).
Documentation comment explains the distinction. ✓

## Verification

Command: `cargo test -p velvet-ballistics --test cli_integration --test finish_input_execution --test admission_evidence_integration`

Results (via immutable runner `/tmp/vb-run-gate.py`):
- exit_code: 0
- seconds: 13.307
- cli_integration: 123 passed; 0 failed
- finish_input_execution: 1 passed; 0 failed
- admission_evidence_integration: 25 passed; 0 failed (includes all modified/new tests)

**Finding 8 (evidence):** All 149 expected test cases pass. Evidence archived at
`.beads/vb-d1068/evidence/cli-regressions.{stdout,stderr,json}`. ✓

**Finding 9 (evidence):** Clippy strict (`-Dwarnings`) passes with exit 0. Evidence
archived at `.beads/vb-d1068/evidence/clippy-strict.{stdout,stderr,json}`. ✓

## Remaining Limitations

- Full Kani/Flux/Verus fleet not run (per instructions: targeted review only)
- No test coverage added for Journaled policy forged-digest rejection (Relaxed+Strict
  coverage is sufficient per policy hierarchy)
- No performance benchmark added (not required for correctness fix)

## Conclusion

Test changes are correct, minimal, and well-targeted. The three modified tests now use
genuine compiled workflows where appropriate, and the new regression test captures the
typed evidence contract (forged digests reject under all policies). All 149 regression
tests pass. Ready for handoff.
