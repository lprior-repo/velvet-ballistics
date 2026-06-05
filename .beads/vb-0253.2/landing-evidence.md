# Landing Evidence - vb-0253.2

## Bead Information
- **Bead ID**: vb-0253.2
- **Title**: Finish ingress modularization and dedupe
- **Landing Date**: 2026-06-05
- **Status**: CLOSED

## Merge Information
- **Merge Commit**: `5ba93c4ddc9375cd85c1d21d5419202d228a9816`
- **Landed via**: merge commit from `8d926bbc288b4fc7ee95b7a0a2d63aaf7e180969`
- **Parent Main**: `5ba93c4ddc9375cd85c1d21d5419202d228a9816`
- **Current main**: `aebe78f36` (origin/main)

## Scoped Quality Gates (vb_ipc)
| Gate | Command | Result |
|------|---------|--------|
| Check | `rtk cargo check -p vb_ipc` | PASS |
| Test | `rtk cargo test -p vb_ipc` | PASS (631 passed) |
| Clippy | `rtk cargo clippy -p vb_ipc --lib -- -D warnings` | PASS |
| Kani | `cargo kani -p vb_ipc --harness kani_ipc_header_decode_valid --quiet` | PASS |

## Global Debt (Not Introduced by vb_ipc)
- `moon ci` fails on out-of-scope global debt:
  - `xtask/src/forbidden_scan.rs` format/lint
  - `crates/vb_storage/tests/recovery_bdd_tests.rs` unused warnings
  - `vb_cli` mode-module/import drift
- This debt is pre-existing and not introduced by vb_ipc changes

## Implementation Summary
- One canonical `MemoryIngress`, `IngressFrame`, `QueueCapacity`, `MaxPayloadBytes`, `BoundedPayload`, and `IpcError` implementation remains
- `crates/vb_ipc/src/lib.rs` reduced to 58 lines (facade/re-export layer)
- Public `vb_ipc` symbols available through facade re-exports
- Duplicate definitions removed from `bounded.rs`, `error.rs`, `ingress.rs`

## Artifacts Verified
- [x] `.beads/vb-0253.2/final-evidence-decision.md` -> STATUS: APPROVED
- [x] `.beads/vb-0253.2/machine-gate-report.md` -> STATUS: APPROVED_WITH_GLOBAL_DEBT
- [x] `.beads/vb-0253.2/truth-serum-report.md` -> STATUS: APPROVED
- [x] `.beads/vb-0253.2/assurance-bundle.md` -> complete

## Bookmark Status
- Bookmark: `go-skill-p0-vb-0253-2`
- Note: Work landed via merge commit to main; bookmark reference preserved in landing documentation

## Bead Close Record
- Closed via: `bd close vb-0253.2`
- Close reason: Landed to origin/main via merge commit 5ba93c4ddc9375cd85c1d21d5419202d228a9816 from 8d926bbc288b4fc7ee95b7a0a2d63aaf7e180969

---

**STATE**: 16 (LANDED)
