# Error Taxonomy: Digest Coverage for Together

bead_id: vb-xi2f.29
bead_title: P1: digest covers together semantics
phase: 3 (rust-contract)
created_at: 2026-05-24

## Error Classification Framework

All errors are classified on two axes:
1. **Detection time**: Compile-time (detected during `cargo build`/`clippy`/Kani) vs test-time (proptest/unit/BBD) vs runtime (production digest collision).
2. **Impact**: Silent (incorrect behavior produced without error) vs noisy (error/crash/assertion failure).

## Error Variants

### ERR-DIGEST-001: Canonical Name Mismatch — Together → "parallel"
- **Severity**: HIGH
- **Detection**: Compile-time (Kani harness `canonical_name_together_harness` currently FAILS)
- **Impact**: Silent — all together workflows share the same digest prefix string, reducing digest entropy
- **Root cause**: `canonical_primitive_name(Together { .. })` returns `"parallel"` at `part_05.rs:105`
- **Fix**: Change line 105 from `=> "parallel"` to `=> "together"`
- **Evidence**: existing Kani proof `kani_canonical_name.rs:42-62`

### ERR-DIGEST-002: Branch Labels Not Hashed
- **Severity**: HIGH
- **Detection**: Test-time (proptest comparing digests with different labels)
- **Impact**: Silent — two workflows with different branch labels produce identical digests
- **Root cause**: `digest_step_primitive` handles `Together` via the fallback `other => hasher.update(canonical_primitive_name(other).as_bytes())` — no structural details accessed
- **Fix**: Add explicit `Together` match arm in `digest_step_primitive` that hashes branch labels

### ERR-DIGEST-003: Branch Count Not Hashed
- **Severity**: HIGH
- **Detection**: Test-time (proptest comparing digests with different branch counts)
- **Impact**: Silent — adding a branch does not change digest
- **Root cause**: Same as ERR-DIGEST-002
- **Fix**: Hash `branches.len() as u16` in the `Together` arm

### ERR-DIGEST-004: Sub-Step Contents Not Hashed
- **Severity**: HIGH
- **Detection**: Test-time (proptest comparing digests with different sub-step primitives)
- **Impact**: Silent — changing a Set to a Do inside a branch does not change digest
- **Root cause**: `source.steps()` returns only top-level steps; nested sub-steps in `TogetherBranch.steps` are never iterated
- **Fix**: Add recursive `digest_sub_step` traversal for branch sub-steps

### ERR-DIGEST-005: Branch Ordering Non-Determinism Potential
- **Severity**: MEDIUM
- **Detection**: Test-time (proptest with branch reordering)
- **Impact**: Silent — same as ERR-DIGEST-002 through ERR-DIGEST-004 but specifically about ordering
- **Root cause**: Since nothing is hashed, ordering is trivially insensitive. After branch labels/steps are added, the `Vec` iteration order must be respected.
- **Fix**: Ensure branches are hashed in array order (inherent from `for branch in branches` iteration)

### ERR-DIGEST-006: Deeply Nested Together Overflow (Future)
- **Severity**: LOW (prevented by validation)
- **Detection**: Compile-time (validation rejects excessive depth)
- **Impact**: Noisy — validation error prevents compilation
- **Root cause**: If `MAX_CONSTRUCT_DEPTH` is exceeded, the workflow is rejected before digest computation
- **Fix**: No fix needed; validation guard is sufficient. Digest recursion uses the same bound.

### ERR-DIGEST-007: Digest Collision Between Semantically Different Workflows
- **Severity**: CRITICAL (if it occurs after fix)
- **Detection**: Proptest (generate many workflow variants, check digest uniqueness)
- **Impact**: Silent — two different together configurations produce same digest
- **Root cause**: Potential bLAKE3 collision (negligible probability) or incomplete hashing after fix
- **Mitigation**: Proptest with coverage analysis verifies all fields reach the hasher

### ERR-DIGEST-008: Dead Code Divergence
- **Severity**: LOW (until code is revived)
- **Detection**: Manual review
- **Impact**: Silent — `compile/mod.rs` contains duplicate `canonical_digest` and `canonical_primitive_name` that are not compiled. If this module is ever linked, the bugs are replicated.
- **Fix**: Delete dead code or add `#[cfg(test)]` annotations with a `compile_error!` guard

### ERR-DIGEST-009: Missing Test — No Together Digest Sensitivity Test
- **Severity**: HIGH
- **Detection**: Test gap analysis
- **Impact**: Silent — bugs can be introduced without test failure
- **Root cause**: No existing test verifies that together semantic changes produce different digests
- **Fix**: Add unit tests and proptest properties

### ERR-DIGEST-010: Aggregate Canonical Name Mismatch (Adjacent Bug)
- **Severity**: MEDIUM (out of scope but noted)
- **Detection**: Kani harness `canonical_name_aggregate_harness` FAILS
- **Impact**: Silent — `Aggregate` maps to `"aggregate"` instead of `"reduce"`
- **Note**: This is a peer bug to ERR-DIGEST-001. It affects a different primitive and is not in scope for this bead.
- **Root cause**: `canonical_primitive_name(Aggregate { .. })` returns `"aggregate"` at `part_05.rs:107`
- **Fix**: Change line 107 from `=> "aggregate"` to `=> "reduce"`

## Severity Summary

| Error | Severity | Detection | Status |
|-------|----------|-----------|--------|
| ERR-DIGEST-001 | HIGH | Kani (failing) | Backlog |
| ERR-DIGEST-002 | HIGH | Proptest (missing) | Backlog |
| ERR-DIGEST-003 | HIGH | Proptest (missing) | Backlog |
| ERR-DIGEST-004 | HIGH | Proptest (missing) | Backlog |
| ERR-DIGEST-005 | MEDIUM | Proptest (missing) | Backlog |
| ERR-DIGEST-006 | LOW | Validation (existing) | Mitigated |
| ERR-DIGEST-007 | CRITICAL | Proptest (missing) | Backlog |
| ERR-DIGEST-008 | LOW | Review | Backlog |
| ERR-DIGEST-009 | HIGH | Test gap | Backlog |
| ERR-DIGEST-010 | MEDIUM | Kani (failing) | Out of scope |
