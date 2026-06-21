# SJ-004: `workflow_digest_bytes_equal` expands a 32-byte compare into ~100 lines

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_storage/src/recovery/digest.rs:7`
- **Confidence**: confirmed

## Description

`workflow_digest_bytes_equal` destructures two `[u8; 32]` arrays into 64
named bindings and writes out 32 short-circuiting `&&` comparisons by hand.
The result is identical to `left == right` (the compiler already lowers array
equality to `memcmp`), and the short-circuit means it is not even
constant-time.

## Evidence

```rust
pub(crate) fn workflow_digest_bytes_equal(left: WorkflowDigest, right: WorkflowDigest) -> bool {
    let [l0, l1, l2, l3, l4, l5, l6, l7, l8, l9, l10, l11, l12, l13, l14, l15,
         l16, l17, l18, l19, l20, l21, l22, l23, l24, l25, l26, l27, l28, l29,
         l30, l31] = left.as_bytes();
    let [r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15,
         r16, r17, r18, r19, r20, r21, r22, r23, r24, r25, r26, r27, r28, r29,
         r30, r31] = right.as_bytes();

    l0 == r0 && l1 == r1 && l2 == r2 && l3 == r3 && l4 == r4 && l5 == r5
        && l6 == r6 && l7 == r7 && l8 == r8 && l9 == r9 && l10 == r10
        && l11 == r11 && l12 == r12 && l13 == r13 && l14 == r14 && l15 == r15
        && l16 == r16 && l17 == r17 && l18 == r18 && l19 == r19 && l20 == r20
        && l21 == r21 && l22 == r22 && l23 == r23 && l24 == r24 && l25 == r25
        && l26 == r26 && l27 == r27 && l28 == r28 && l29 == r29 && l30 == r30
        && l31 == r31
}
```

The same file already proves the simpler form works: `recover.rs:155` and
`admission.rs:155` (`verify_admission_digest`, `verify_policy_expectations`)
compare `WorkflowDigest` values directly with `==`. There is no constant-time
requirement here (these are storage-layer digest equality checks, not
secret-token checks) and the function is `pub(crate)`.

## Adversarial Check

One might argue the manual expansion was an attempt at constant-time
comparison to defeat timing attacks. It is not — the `&&` chain short-circuits
on the first mismatched byte, leaking the position of the first difference.
So the function provides neither readability nor security. If constant-time
comparison were actually required, `subtle::ConstantTimeEq` would be the
correct primitive.

## Suggested Fix

Replace the entire body with:
```rust
pub(crate) fn workflow_digest_bytes_equal(left: WorkflowDigest, right: WorkflowDigest) -> bool {
    left == right
}
```
If constant-time comparison is ever needed, switch to
`subtle::ConstantTimeEq` from the `subtle` crate — do not hand-roll.
