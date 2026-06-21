# RA-010: Misleading "widening conversion" comment for `MAX_STEPS_PER_WORKFLOW as u32`

- **Severity**: Info
- **Category**: correctness (misleading comment / lint allow)
- **Location**: `crates/vb_runtime/src/admission/admission.rs:232-240`
- **Confidence**: confirmed

## Description

`per_workflow_step_ceiling` annotates its `usize as u32` cast with a comment claiming it is a "widening conversion that cannot lose data." On the project's target platforms `usize` is 64-bit, so the cast is in fact *narrowing*; it only happens to be lossless because of the upstream invariant `MAX_STEPS_PER_WORKFLOW <= usize::from(u16::MAX)` (`vb_core/src/limits/tests.rs:272`). The clippy allows on the function are thus working against an incorrect comment.

## Evidence

```rust
#[must_use]
pub const fn per_workflow_step_ceiling() -> u32 {
    // The master contract ceiling lives in `vb_core::limits::MAX_STEPS_PER_WORKFLOW`
    // and is bounded by `usize::from(u16::MAX)` per the limits test suite, so
    // the `u32` cast is a widening conversion that cannot lose data.
    #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
    let ceiling = vb_core::limits::MAX_STEPS_PER_WORKFLOW as u32;
    ceiling
}
```

`MAX_STEPS_PER_WORKFLOW: usize = 1_000` (`vb_core/src/limits.rs:12`). On 64-bit Linux (the project's target per AGENTS.md "Platform: linux"), `usize = u64`, so `usize as u32` truncates the high 32 bits. The value-invariant rescue is `MAX_STEPS_PER_WORKFLOW <= u16::MAX`, which the comment does mention but then mis-summarizes as "widening."

## Adversarial Check

One could argue this is "just a comment" and has no runtime effect. True, but the engineering rules in AGENTS.md forbid "as numeric casts" in production and the function works around the rule via `#[allow(...)]`. The justification for the allow is the comment, and the comment is wrong about the cast direction. A future agent editing this code could plausibly remove the upstream invariant test (`limits/tests.rs:272`) believing the cast is structurally safe, when in fact the test is the only thing making it safe. The correct framing is "narrowing cast guarded by upstream invariant."

## Suggested Fix

Replace `as u32` with the lossless `u32::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW).unwrap_or(u32::MAX)` (const-evaluable since Rust 1.65), and delete the clippy allow + misleading comment. Or rewrite the constant as `u32` upstream in `vb_core::limits.rs` so the cast disappears entirely.
