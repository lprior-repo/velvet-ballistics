# vb-cycog: withdrawn mistaken dormant-test diagnosis

All four tests were ALREADY explicitly wired in crates/velvet_ballistics/Cargo.toml at ebe09843, 52a69710 and main d0645504. Inventory limited to velvet-ballistics-workspace-tests excluded tests owned by the binary package; that omission was not a production defect. Native3090 only renamed existing targets. Those unnecessary Cargo changes have been reverted byte-exact to base. No Rust source changed. The new-name run V2 passed11 tests across4 binaries; the original-name run is required next. No restoration or newly activated coverage may be claimed.

Original target run V3 PASS:11 tests across4 actual binaries,0 skipped,0.259s (raw log authoritative duration). All four original targets execute against real Cargo-built binary. Bead invalid, no source change landed.
