# Trusted Base Plan — vb-rpch TLC Fix Pass

## Trusted Components

- TLC executable discovered at `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`.
- Java executable discovered at `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java` for fallback only if a real `tla2tools.jar` path is found later.
- Finite model bounds in cfg files.
- Fidelity of the abstraction between `RecoveryReplayFull.tla` events and Rust journal/recovery semantics remains a later bridge/review concern.

## Untrusted/Stale Until Re-executed

- Root `proof-review.md` and `.beads/vb-rpch/proof-review.md` APPROVED status.
- `.beads/vb-rpch/machine-gate-report.md` exhaustive 443k claim.
- Any old `tlc-fixed.log` unless matched to current spec/cfg hashes and final completion output.
- `RecoveryErrorExhaustive` unless each error value has reachability evidence.

## Required Bound Disclosure

- Smoke cfg: `RunId={1}`, `StepId={1}`, `ActionId={1}`, `Attempt={1}`, `MAX_SEQ=3`, `MAX_EVENTS=3`.
- Primary cfg: current `RunId={1,2}`, `StepId={1,2,3}`, `ActionId={1,2}`, `Attempt={1,2}`, `MAX_SEQ=100`, `MAX_EVENTS=20`.
- If primary run leaves states on queue or is stopped, reports must say `PARTIAL_BFS`, not exhaustive PASS.

## Waiver Candidates

- No behavior-affecting waiver is accepted by this planner pass.
- Possible non-behavior waiver candidate only: primary cfg state-space explosion, if and only if smoke and non-vacuity runs complete and the large run records raw partial BFS statistics.
