# Handoff — vb-eq4ff recovery_verification Verus ICE repair

Author: DeepSeek v4Pro (repair author)
Workspace: `/home/lewis/src/isoloated/velvet-ballistics-vb-eq4ff-deepseek-verus-repair`
Git root == JJ root == workspace (verified). Coordinator owns VCS commit/bookmark/push.
Bead left **in_progress** for independent review + main.

## Files changed (exact recovery ICE files only)

| File | Change |
|------|--------|
| `verification/verus/recovery_verification.rs` | `CANNOT_RESUME_REASONS` const moved OUTSIDE `verus!` (plain Rust, exact 13 tokens); removed in-macro re-export |
| `verification/verus/extern_recovery_verification.rs` | removed in-macro `pub const CANNOT_RESUME_REASONS: [&str; 13]` + doc |

## Evidence

- Raw failure: reproduced pre-fix (ICE, exit 1) — matches canonical frozen
  baseline `.beads/vb-2lyiw/evidence/canonical-frozen-baseline.stdout` line ~5028.
- Raw success (immutable runner): `.beads/vb-eq4ff/evidence/recovery-verification-success.json`
  `{"exit_code":0,"argv":["verus","--crate-type=lib","verification/verus/recovery_verification.rs"]}`;
  stdout `verification results:: 123 verified, 0 errors`.

## Constraints honored

No `--no-lifetime`; lifetime checks on; no checker exclusion; no weakened proof
contract; exact production binding/semantics preserved (const token order identical).
Upstream issue #2323 (async Send) noted as background only — not the cause here.

## Residual / limits

- `verify-production-inner-drift` gate still reports unrelated drift (RecoveredStepEntry,
  RecoveredPendingAction, hydrate_run_frame, etc.) — NOT in this bead scope.
- Not self-approved. Independent review (vb-fg1pi surface untouched except the two
  named recovery ICE files).
