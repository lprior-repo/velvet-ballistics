reviewer_skill: proof-reviewer
reviewer_invocation_id: inv-proof-reviewer-r2-vb-xi2f-compile-source
reviewer_state: 6
round: 2
target: verification/verus/vb_xi2f_compile_source.rs
round1_finding_under_review: compile_source_pure is an in-spec projection, not drift-checked
planner_invocation_id: inv-proof-planner-s4

# Round 2 Proof Review: vb-xi2f_compile_source.rs

## Required Fix (from Round 1)

The Round 1 reviewer identified that the spec file's `compile_source_pure`
projection (declared in-file as `#[verifier::external]`) was a hand-written
shadow of the 7-step production chain at
`crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60` and that
this projection was not drift-checked. Round 2 was required to:

1. Create a proper `production_inner/vb_xi2f_compile_source_production.rs`
   mirror of the 7-step production chain.
2. Add a DRIFT POLICY header to that mirror.
3. Run `scripts/check-production-inner-drift.sh` and confirm 0 drift
   findings on the new mirror.

## Evidence Collected

### E1. Mirror file existence
- Path checked: `verification/verus/production_inner/vb_xi2f_compile_source_production.rs`
- `ls -la` output: `No such file or directory`
- `ls verification/verus/production_inner/ | grep -i 'compile_source'` output:
  `no compile_source mirror found`
- `ls verification/verus/production_inner/ | grep -i 'xi2f'` output:
  `no xi2f files in production_inner`

### E2. Production-binding audit
- `bash scripts/check-verus-production-binding.sh` output:
  `STRONG (direct crates/ binding): 3`
  `WEAK (production_inner/ mirror): 67`
  `VACUUM (no production binding):  0`
- The spec file IS classified as WEAK via the companion-extern pattern:
  `#[path = "extern_try_from_parts.rs"]` in spec →
  `#[path = "production_inner/try_from_parts_production.rs"]` in extern.
- However, this binding only covers step 8 of the production chain
  (`CompiledWorkflow::try_from_parts`). Steps 1-7 (validate_canonical_compile_scope,
  validate_branch_counts, EmptySteps check, canonical_layout, lower_canonical_step
  loop, WorkflowParts construction, vb_validate::shared::validate) are
  encoded as in-spec Rust inside `compile_source_pure`'s
  `#[verifier::external]` body and have NO production mirror.

### E3. DRIFT POLICY header
- `grep -n "DRIFT POLICY" verification/verus/vb_xi2f_compile_source.rs`:
  no matches.
- The spec file does not claim a DRIFT POLICY on the compile_source chain;
  the in-spec projection body explicitly admits this as binding debt
  ("drift between this body and the production body is binding-debt
  tracked outside Verus", spec file line 212).

### E4. Verus smoke
- `verus --crate-type=lib verification/verus/vb_xi2f_compile_source.rs`:
  `verification results:: 15 verified, 0 errors`
- Smoke pass only proves the math is internally consistent. It does NOT
  prove the in-spec projection matches the production body.

### E5. Drift gate
- `bash scripts/check-production-inner-drift.sh` output:
  `Mirror files checked: 58`
  `Drift findings:       0`
  `Production-inner drift gate: PASS`
- `grep -i 'xi2f\|compile_source' target/verus-drift/drift.log`: no
  matches.
- The gate runs on the 58 existing mirrors; the new compile_source mirror
  is not present, so the gate has nothing to check. "PASS" here is
  vacuous for the 7-step chain.

## Verdict: VACUUM (Round 2 fix not performed)

The Round 2 fix was NOT performed:

1. `production_inner/vb_xi2f_compile_source_production.rs` does not exist.
2. No DRIFT POLICY header was added.
3. The drift gate was not exercised against the new mirror.

The spec file's `compile_source_pure` projection remains an in-spec
`#[verifier::external]` body. Verus sees the contract via
`assume_specification`; the body is opaque. The spec is bound to
production ONLY for step 8 (`try_from_parts`) via the existing
`extern_try_from_parts.rs` -> `production_inner/try_from_parts_production.rs`
WEAK chain. Steps 1-7 of the production chain are un-verified shadows.

## Required Fix for Round 3

1. Create `verification/verus/production_inner/vb_xi2f_compile_source_production.rs`
   that mirrors the 7-step production chain at
   `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60` (and the
   supporting types `CanonicalStepLayout`, `canonical_layout`,
   `canonical_step_width`, `CompileError`, `CompileErrors` from
   `mod_compile_errors`, etc.). The mirror must:
   - Be verbatim where possible, with documented `// SUBSTITUTED:` and
     `// REMOVED:` lines for every substitution.
   - Carry a `DRIFT POLICY` header citing the production file:line range.
   - Carry per-section `// Production path:start-end` annotations.
   - Include a phantom drift detector at the bottom of the file that
     resolves every production method name referenced in the chain.
2. Update `verification/verus/vb_xi2f_compile_source.rs` to bind
   `compile_source_pure` via `#[path =
   "production_inner/vb_xi2f_compile_source_production.rs"]` and remove
   the in-spec `#[verifier::external]` projection body, OR introduce a
   companion `extern_vb_xi2f_compile_source.rs` that performs the same
   `#[path]` binding.
3. Update the spec's TRUST BOUNDARY section to declare the new binding
   and remove the "binding-debt tracked outside Verus" admission.
4. Re-run `bash scripts/check-production-inner-drift.sh` and confirm the
   new mirror produces 0 drift findings.
5. Re-run `bash scripts/check-verus-production-binding.sh` and confirm
   the spec is classified WEAK (mirror) directly, not via the
   companion-extern delegation through `try_from_parts`.

## Findings

```yaml
- finding/v1:
    id: F-R2-001
    severity: blocker
    artifact: verification/verus/vb_xi2f_compile_source.rs
    obligation: PO-001
    description: |
      Round 2 fix not performed. The required
      production_inner/vb_xi2f_compile_source_production.rs mirror does
      not exist. The 7-step production chain in
      crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60 is
      encoded as an in-spec #[verifier::external] body with NO drift
      check. The Round 1 finding ("compile_source_pure was an in-spec
      projection, not drift-checked") is unresolved.
    raw_evidence:
      - 'ls verification/verus/production_inner/vb_xi2f_compile_source_production.rs: No such file or directory'
      - "ls verification/verus/production_inner/ | grep -i 'compile_source': no compile_source mirror found"
      - "ls verification/verus/production_inner/ | grep -i 'xi2f': no xi2f files in production_inner"
      - "grep -n 'DRIFT POLICY' verification/verus/vb_xi2f_compile_source.rs: no matches"
      - "bash scripts/check-production-inner-drift.sh: Mirror files checked: 58 (compile_source mirror absent)"
    required_fix: |
      Create the production_inner mirror with DRIFT POLICY header as
      described in the Required Fix for Round 3 section above. Re-run
      the drift gate and the production-binding audit.
    disposition: blocker

- finding/v1:
    id: F-R2-002
    severity: blocker
    artifact: verification/verus/vb_xi2f_compile_source.rs
    obligation: PO-001
    description: |
      Spec file's binding classification is WEAK only because of the
      transitive delegation through extern_try_from_parts.rs ->
      try_from_parts_production.rs. The delegation covers only step 8
      of the production chain. The compile_source_pure body itself
      (steps 1-7) is unbound to production.
    raw_evidence:
      - "spec.rs line 72: #[path = \"extern_try_from_parts.rs\"]"
      - "extern_try_from_parts.rs line 114: #[path = \"production_inner/try_from_parts_production.rs\"]"
      - "Spec's compile_source_pure body (lines ~214-300+) is #[verifier::external] with no path attribute"
    required_fix: |
      Either bind compile_source_pure directly to a production_inner
      mirror (STRONG/WEAK pattern), or document why the in-spec
      projection is acceptable for steps 1-7.
    disposition: blocker

- finding/v1:
    id: F-R2-003
    severity: major
    artifact: verification/verus/vb_xi2f_compile_source.rs
    obligation: PO-001
    description: |
      The spec file explicitly admits binding debt in its TRUST BOUNDARY
      section: "drift between this body and the production body is
      binding-debt tracked outside Verus" (line 212). This admission
      confirms the Round 1 finding is unaddressed and the in-spec
      projection has no drift gate.
    raw_evidence:
      - "Spec line 212: '// production decision in `?`-propagation order; drift between this'"
      - "Spec line 213: '// body and the production body is binding-debt tracked outside Verus.'"
    required_fix: |
      Remove the binding-debt admission by binding the projection to a
      production_inner mirror, or document a waiver with owner, expiry,
      and compensating evidence.
    disposition: blocker
```

## Output (required format)

```
file: verification/verus/vb_xi2f_compile_source.rs
verdict: VACUUM
mirror_created: no
mirror_drift_policy: no
verus_smoke: 15 verified, 0 errors
drift_finding: "verification/verus/production_inner/vb_xi2f_compile_source_production.rs does not exist; Round 2 fix not performed; the in-spec #[verifier::external] body of compile_source_pure (steps 1-7 of the production chain) remains unbound to production"
recommendations:
  - "Create verification/verus/production_inner/vb_xi2f_compile_source_production.rs as a verbatim mirror of crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60 with documented SUBSTITUTED/REMOVED lines and a DRIFT POLICY header."
  - "Re-route compile_source_pure in the spec to bind via #[path = \"production_inner/vb_xi2f_compile_source_production.rs\"] (or via a companion extern that does), and remove the in-spec #[verifier::external] body."
  - "Remove the 'binding-debt tracked outside Verus' admission in the TRUST BOUNDARY section once the binding is real."
  - "Re-run scripts/check-production-inner-drift.sh and scripts/check-verus-production-binding.sh and verify 0 drift findings and direct (not transitive) WEAK classification."
```

STATUS: REJECTED
