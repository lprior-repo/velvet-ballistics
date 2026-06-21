# CV-105: Diagnostic category ranges collide on E13xx

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/diagnostic/codes.rs:40`
- **Confidence**: confirmed

## Description

The diagnostic category model assigns `E13xx` to Accessor, Internal, and execution/resource errors. Public fallback classification maps all unregistered `0x13xx` values to `Accessor`, so execution/internal codes in that range are category-ambiguous unless every exact registry entry is present and correct.

## Evidence

The category comments assign `E13xx` to Accessor and Internal:

```rust
/// Accessor and path errors: E13xx
Accessor,
...
/// Internal invariant violations (fallback codes): E13xx
Internal,
```

Execution errors also allocate `0x13xx` values:

```rust
pub(super) const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x1301);
pub(super) const RESOURCE_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1302);
pub(super) const ALLOCATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x1303);
pub(super) const EXPRESSION_STACK_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x1304);
...
pub(super) const INTERNAL_INVARIANT_CODE: DiagnosticCode = DiagnosticCode::new(0x1309);
```

The fallback classifier chooses Accessor for the whole high-byte range:

```rust
0x13 => super::codes::CodeCategory::Accessor,
```

## Adversarial Check

The registry-first lookup can mask this for entries that are registered with explicit categories, but the public helper intentionally falls back to high-byte heuristics for unregistered numeric codes. The stated invariant in `codes.rs` says category consistency is based on the high byte; that invariant cannot hold when multiple domains share `0x13xx`.

## Suggested Fix

Give Accessor, Internal, and execution/resource errors disjoint high-byte ranges, or remove high-byte fallback classification for ambiguous ranges and require exact registry entries for every supported code.
