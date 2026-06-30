# Contract — vb-xkli

STATUS: APPROVED

Requirement: the P0 Kani repair is acceptable only if the repository proof gauntlet's scripted Kani harnesses pass without weakening flags or skipped harness failures.

Acceptance boundary: this bead does not claim whole-repo Kani coverage from `cargo kani list`; it claims the exact scripted P0 proof lane executed by `scripts/rust-verification-gauntlet.sh proof`.
