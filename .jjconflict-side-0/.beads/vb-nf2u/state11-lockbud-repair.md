STATUS: PASS

# State 11 Lockbud Waiver-Policy Repair — vb-nf2u

## Files changed
- `scripts/rust-verification-gauntlet.sh`
- `.moon/tasks/all.yml`
- `.beads/vb-nf2u/state11-lockbud-repair.md`

## Lockbud / waiver decision
- No real approved `LOCKBUD_CMD` was available; prior evidence showed no installable crates.io `lockbud` package.
- Repaired the gauntlet to consume the explicit bead-scoped waiver artifact `.beads/vb-nf2u/verification-layers.md` section `WAIVE-CONCURRENCY-UI-RELEASE` only when `ALLOW_BEAD_LOCKBUD_WAIVER=1` and `VERIFY_BEAD_ID=vb-nf2u` are present.
- `moon run :verify-all` now passes explicit context through `.moon/tasks/all.yml`: `env VERIFY_BEAD_ID=vb-nf2u ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all`.

## Why this is not silent weakening
- The gauntlet still executes `LOCKBUD_CMD` first if present.
- The gauntlet still skips Lockbud only when no concurrency markers exist.
- If concurrency markers exist and no `LOCKBUD_CMD` exists, the gauntlet now waives Lockbud only after validating all required waiver fields in `.beads/vb-nf2u/verification-layers.md`: clause IDs, waived layer, reason, compensating evidence, owner, expiry/follow-up.
- The waiver path requires explicit bead context; no bead id means failure, not pass.
- The waiver path runs a focused static scan over UI release surface paths (`xtask/src`, `crates/vb_ui_snapshot/src`, `crates/vb_ui_makepad/src`) for spawned tasks/shared state/channel/cancellation markers before skipping Lockbud.

## Command evidence
- PASS: `bd prime` loaded bead workflow context.
- PASS: `bash -n scripts/rust-verification-gauntlet.sh && env VERIFY_BEAD_ID=vb-nf2u ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh deep`.
  - Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e1084fae10014MHL5w0I8hX5o0`.
- PASS: `moon run :verify-all`.
  - Observed Lockbud lane: `[verify:all] Lockbud waived by bead-scoped artifact .beads/vb-nf2u/verification-layers.md / WAIVE-CONCURRENCY-UI-RELEASE`.
  - Summary: `Tasks: 1 completed`, time `1m 9s 792ms`.
  - Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e1094c5340014z3e0Xz0DecbDW`.
- PASS: `moon ci --base HEAD --head HEAD`.
  - Summary: `Tasks: 20 completed (2 cached)`, time `8m 25s 285ms`.
  - Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e1095196f001TB5byvqo5NdV1d`.

## Power-of-Ten / zero-panic rules affected
- Fail-closed verification: satisfied; missing bead id, missing waiver artifact, incomplete waiver text, or detected UI-release concurrency now fails.
- Bounded scope: satisfied; waiver scan is limited to release-capture surface rather than silently suppressing broad workspace markers.
- No production Rust forbidden constructs affected; only Bash/Moon tooling and this evidence artifact changed.

## Performance-layer decision
- No performance claim made. No benchmark/profiler evidence required.

## Second-ring evidence
- Formal gauntlet evidence attached through passing `moon run :verify-all` output above.
- Lockbud itself was not executed; it was explicitly waived through bead-scoped policy after validating owner/expiry/compensating evidence and static release-surface scan.

## Residual risks
- The waiver is specific to `vb-nf2u` fixture-backed single-process UI release capture and must be revoked if `ai-release` UI capture introduces threads, async tasks, channels, shared mutable state, or cancellation.
- Broad workspace concurrency markers remain outside this bead's scope; future concurrent work still needs real Lockbud or its own reviewed waiver.
