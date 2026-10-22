# Rust Contract: vb-eq4ff recovery_verification Verus ICE repair

**Bead:** vb-eq4ff
**Type:** Bounded verifier ICE repair (narrow structural fix)
**Date:** 2026-10-21
**Authoritative failure:** canonical-frozen-baseline (vb-2lyiw) `verify-verus` exit 1,
`verification/verus/recovery_verification.rs`.

## Failure

```
Internal Verus Error: The thir_body query is running for item
DefId(0:110 ~ recovery_verification[8fa4]::production::CANNOT_RESUME_REASONS)
which may require erasure, but the VerusErasureCtxt has not been initialized.
```

Reproduced verbatim in this workspace before the fix (`verus --crate-type=lib
verification/verus/recovery_verification.rs`, exit 1).

## Root cause

`pub const CANNOT_RESUME_REASONS: [&str; 13]` is a reference-typed module-level
`const` declared inside the `verus!` macro (via the `#[path]`-bound `production`
module = `extern_recovery_verification.rs`) and re-exported with
`pub use production::{ CANNOT_RESUME_REASONS, ... }`. Verus runs `thir_body`
erasure on the const; a `const` (not an `exec fn`) has no initialized
`VerusErasureCtxt`, so lifetime erasure of the `&str` array ICEs. The const is
not referenced by any proof body (only the re-export).

## Fix (narrow, preserves binding + semantics)

1. `verification/verus/recovery_verification.rs`: declare
   `pub const CANNOT_RESUME_REASONS: [&str; 13]` as **plain Rust OUTSIDE** the
   `verus!` macro (exact 13 tokens, production classification order preserved).
2. Remove the in-macro definition from `extern_recovery_verification.rs` and the
   `CANNOT_RESUME_REASONS` entry from the in-macro `pub use production::{...}`.

No `--no-lifetime`, no Verus downgrade/checker exclusion, no weakened
spec/exec/proof contract, no fake stub. Default lifetime checks stay on.

## Verification

- `python3 /tmp/vb-run-gate.py 300 <prefix> verus --crate-type=lib verification/verus/recovery_verification.rs`
  → exit 0, `verification results:: 123 verified, 0 errors`
  (evidence: `.beads/vb-eq4ff/evidence/recovery-verification-success.{json,stdout,stderr}`).

## Non-goals / untouched

- `verify-production-inner-drift` failures (RecoveredStepEntry etc.) are a
  separate gate, not touched.
- Other Verus proof mirrors (vb-fg1pi review surface) untouched except the two
  recovery ICE files named above.
