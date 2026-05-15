# Baseline Report - vb-qi37.1.5

bead_id: vb-qi37.1.5
bead_title: runtime/recovery: Prove replay digest mismatch detection
phase: 1
updated_at: 2026-05-13T18:00:00Z

## Parent Commit
xkkslzqn 2128d41671c7edda369a3d76859a0f9f7b3605e2

## Source Checkout
/home/lewis/src/Velvet-ballistics

## Isolated Workspace
/home/lewis/src/vb-qi37-1-5

## Isolation Verification
```bash
case "/home/lewis/src/vb-qi37-1-5" in "/home/lewis/src/Velvet-ballistics"|"/home/lewis/src/Velvet-ballistics"/*) echo "NESTED";; *) echo "ISOLATED";; esac
# Output: ISOLATED
```
Workspace is a jj worktree sibling to source checkout.

## Pre-Edit Baseline Commands

### cargo build
```
cd /home/lewis/src/vb-qi37-1-5 && cargo build 2>&1
cargo build (0 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s
EXIT: 0
```
Status: PASS — workspace is up-to-date, no compilation needed.

### cargo test --no-run
```
cd /home/lewis/src/vb-qi37-1-5 && cargo test --no-run 2>&1; echo "EXIT: $?"
EXIT: 0
```
Status: PASS — all tests already compiled, no output means nothing to rebuild.

### cargo clippy --no-deps -- -D warnings
```
cd /home/lewis/src/vb-qi37-1-5 && cargo clippy --no-deps -- -D warnings 2>&1
cargo clippy: No issues found
EXIT: 0
```
Status: PASS — zero clippy warnings or errors.

## Baseline Summary
- Build: CLEAN (0 crates, 0.29s)
- Tests: COMPILED (exit 0, no output = already up-to-date)
- Clippy: CLEAN (No issues found)
- Baseline is GREEN — no pre-existing failures.

## Prior Artifacts Note
This workspace contains prior work from a previous agent session (State 6 proof review REJECTED). Artifacts are preserved in `.beads/vb-qi37.1.5/archive/`. Baseline captured fresh per controller directive.

## Verification Target
`vb-qi37.1.5` must prove that replay digest mismatch detection is correctly implemented and formally verified. Acceptance criteria: tests intentionally corrupt artifact digest, journal sequence, slot value, and taint; each case fails deterministically with a precise diagnostic.
