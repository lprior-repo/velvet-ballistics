# Independent Code and Test Review: vb-08daa

**Reviewer:** RTX3090 HIGH (local-3090-llama/qwen3.8-27b-uncensored)
**Date:** 2026-10-22
**Worktree:** /home/lewis/src/isoloated/vb-08daa-clippy-independent-review-v1
**Base:** b9ff52cb98b3136934a353a2c7691cc5420517b6
**Parent:** 42fd94f2

## Summary

Recovered 3 of 4 hunks from `.beads/vb-08daa/historical-own-patch.patch`. The 4th hunk (`&source.inputs()` → `source.inputs()`) was already present in `crates/vb_compile/src/mod_compile_lowering/part_01.rs` at line 57. All recovered hunks match the historical patch exactly.

## Code Review

### Hunk 1: `.cargo/config.toml` (ci alias) — ✓ Correct

- **Change:** `--all-targets` → `--lib --bins --examples` in `ci` alias
- **Line:** 18
- **Rationale:** AGENTS.md states "tests compile and run, but test clippy is not strict." The ci alias should target source code only.
- **Invariants preserved:** `--workspace`, `--all-features`, `-- -D warnings` all retained
- **Verification:** Matches `lint-src` moon task target selection at `.moon/tasks/all.yml:51`

### Hunk 2: `contracts/invariants.yaml` (no_unsafe_in_first_party scan) — ✓ Correct

- **Change:** `--all-targets` → `--lib --bins --examples` in `no_unsafe_in_first_party` scan command
- **Line:** 400
- **Rationale:** Aligns documented invariant with actual `lint-src` moon task implementation
- **Invariants preserved:** Still denies all unsafe code via `-D unsafe_code`; only target selection changed
- **Verification:** Matches `unsafe-audit` moon task which also excludes tests, benches, examples, fixtures, and verification directories

### Hunk 3: `xtask/src/evidence/persistence.rs` (clippy repair hint) — ✓ Correct

- **Change:** `--all-targets --all-features` → `--lib --bins --examples --all-features` in clippy repair command hint
- **Line:** 67
- **Rationale:** Repair hint must match ci alias target selection for consistency
- **Verification:** Called at line 31 in `persist_gate_failure()` for evidence generation; consistent with actual source gate

### Hunk 4: `crates/vb_compile/src/mod_compile_lowering/part_01.rs` — ✓ Already Present

- **Change:** `&source.inputs()` → `source.inputs()` (removal of redundant reference)
- **Line:** 57
- **Verification:** Current code shows `source.inputs()` with no `&`; historical cruft was already removed by earlier pass
- **Note:** No code change required or made; correctly identified as already applied

## Test Applicability Review

### Existing Gate Alignment

All changes align with the existing moon task definitions:

| Moon Task | Target Selection | Status |
|-----------|------------------|--------|
| `lint-src` | `--lib --bins --examples --all-features` | ✓ Matches new ci alias |
| `check` | `--all-targets --all-features` | ✓ Unchanged (type checking) |
| `unsafe-audit` | ripgrep pattern scan, excludes tests/benches/examples/fixtures/verification | ✓ Matches new invariant |
| `panic-surface` | script-based scan | ✓ Unchanged |
| `ignored-fallible-results` | script-based scan | ✓ Unchanged |

### No New Tests Required

These are configuration and contract changes only. No runtime behavior, API surface, or source attribute modifications. Verification relies on existing source gates.

### Documentation Discrepancy (Noted, Out of Scope)

`docs/rust-governance.md` line 70 still shows `cargo clippy --workspace --all-targets --all-features -- -D warnings`. This is a documentation file; contract explicitly states "Master doc and broader docs unchanged per user instruction." Requires future bead for consistency.

## Findings

No P0, P1, or P2 findings. All 3 recovered hunks are correct, match the historical patch, and align with existing source gate implementations.

## Verdict

**APPROVED**

Bounded source and test applicability confirmed:
- Configuration changes only (no runtime-core logic)
- Matches established `--lib --bins --examples` pattern used in `lint-src` moon task
- `check-all` alias and `check` moon task still use `--all-targets` for type checking
- No weakening of no-unsafe policy; target selection aligns with AGENTS.md test clippy policy
- No new tests required; existing source gates cover the scope