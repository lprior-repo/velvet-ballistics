# Kani Verification Layer Report — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10
**Date**: 2026-05-26
**Verifier**: formal-verifier agent
**Tool**: cargo-kani 0.67.0

---

## Summary

**15 Kani proof obligations (PO-001 through PO-015). 0 PASS, 14 FAIL_LOCAL, 1 WAIVED.**

All Kani harnesses are blocked by a compilation error in two harness source files within `crates/vb_core/src/kani/`. The production enum `CodeCategory` gained an `Internal` variant that is not handled in non-exhaustive match expressions in `kani_symbolic_code_validation.rs` and `kani_registry_category.rs`.

Since these files are part of the `vb_core` crate source tree, they are compiled whenever any Kani harness is invoked — including harnesses in `vb_validate`, `vb_yaml`, and `workspace_tests`.

---

## Blocked Harnesses

| PO | Harness | Crate | Prior Status | Block Cause |
|---|---|---|---|---|
| PO-001 | kani_from_static_validation | vb_core | blocked_iter_find_sso | CodeCategory::Internal (kani_symbolic_code_validation.rs) |
| PO-002 | kani_registry_bijection | vb_core | partially_verified | CodeCategory::Internal (kani_registry_category.rs) |
| PO-003 | kani_validation_error_code_registered_1..6 | vb_validate | verified_r9 | CodeCategory::Internal (transitive via vb_core dep) |
| PO-004 | kani_is_supported_code_accepts_ranges | vb_core | partially_verified | CodeCategory::Internal |
| PO-005 | kani_diagnostic_constructor_consistency | vb_core | blocked_iter_find_sso | CodeCategory::Internal |
| PO-006 | kani_yaml_error_code_registered_1..2 | vb_yaml | verified_r9 | CodeCategory::Internal (transitive via vb_core dep) |
| PO-007 | kani_zero_alloc_hot_path | vb_core | waived | WAIVED (WVR-PS010-ALLOC) + compilation |
| PO-008 | kani_from_str_backward_compat | vb_core | blocked_iter_find_sso | CodeCategory::Internal |
| PO-009 | kani_serde_rejects_unknown | vb_core | partially_verified | CodeCategory::Internal |
| PO-010 | kani_registry_nonzero | vb_core | verified_r6 | CodeCategory::Internal |
| PO-011 | kani_registry_category_match | vb_core | verified_r6 | CodeCategory::Internal (own file) |
| PO-012 | kani_reverse_lookup | vb_core | blocked_iter_find_sso | CodeCategory::Internal |
| PO-013 | kani_symbolic_code_determinism | vb_core | blocked_iter_find_sso | CodeCategory::Internal |
| PO-014 | kani_diagnostic_no_mismatch | vb_core | blocked_iter_find_sso | CodeCategory::Internal |
| PO-015 | kani_error_types_symbolic_code | workspace_tests | blocked_workspace_tests | CodeCategory::Internal + xtask |

---

## Compilation Error Detail

```
error[E0004]: non-exhaustive patterns: `diagnostic::CodeCategory::Internal` not covered
  --> crates/vb_core/src/kani/kani_symbolic_code_validation.rs
  --> crates/vb_core/src/kani/kani_registry_category.rs:24:11
  --> crates/vb_core/src/kani/kani_registry_category.rs:38:11
```

These three match expressions need an `Internal` arm. The files are at:
- `crates/vb_core/src/kani/kani_symbolic_code_validation.rs`
- `crates/vb_core/src/kani/kani_registry_category.rs`

---

## Reproduction

```bash
cd /home/lewis/src/vb-workspaces/vb-xi2f.10
cargo kani --harness kani_from_static_validation -p vb_core
# error[E0004]: non-exhaustive patterns: `diagnostic::CodeCategory::Internal` not covered
```

---

## Mitigation

All affected contract clauses are covered by proptest defense-in-depth (PO-016 through PO-026). See `formal-verification-report.md` §2 for proptest results.
