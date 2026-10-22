# Rust Contract: vb-2lyiw Integration Validation

**Bead:** vb-2lyiw
**Type:** Integration validation and bounded source repair
**Date:** 2026-10-21

## Scope

Canonical integration validation of DeepSeek's bounded cleanup (f4a405469) + quality evidence (18162b518) into parent fixtures c816f6b3 and evidence correction 6e3c5375. Conflict-free merge already completed.

## Parent Commits

- c816f6b3 - Final fixture source
- f4a405469 - DeepSeek bounded cleanup
- 6e3c5375 - Fixture evidence correction

## Contract Terms

### 1. Code Review (Completed)
- Reviewed production Flux syntax fix: `16#ffff#` → `u16::MAX` (identical bound 65535)
- Reviewed test safe first getter fix: `events[0]` → `.first().ok_or(...)` with derefs
- Both approved; see `.code-reviews/` directory

### 2. Test Review (Completed)
- Reviewed test assertion preservation
- Same validation semantics, safer accessor
- Approved; see `.code-reviews/` directory

### 3. CI Baseline (Pending)
- Run `moon ci` ONCE as baseline
- Capture full task graph and report
- Identify failing lanes for targeted repair

### 4. Bounded Source Repair
- Repair only actual new integration failures
- No unrelated feature implementation
- No weakening of mathematical obligations
- No test assertion weakening or skips
- No gate config, allow attribute, or gitignore changes

### 5. Final Validation
- Run `moon ci` after repairs
- Verify exact test totals and exit codes
- Write handoff with complete evidence

## Not In Scope

- Go skill or proof planning
- Full fleet mutation testing
- Unrelated code refactoring
- Long-standing pre-existing failures (document but don't silently waive)

## Success Criteria

- `moon ci` passes or all failures explained/bounded
- All touched files documented
- Exact test totals and exit codes recorded
- Handoff complete with evidence artifacts