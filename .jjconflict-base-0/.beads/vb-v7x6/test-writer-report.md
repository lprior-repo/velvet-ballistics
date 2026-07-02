bead_id: vb-v7x6
phase: 8
attempt: 1-of-7

Changed `xtask/tests/ui_release_gates.rs` to resolve `xtask` via `CARGO_BIN_EXE_xtask` when present and fall back to `cargo xtask --` when the nextest/full-workspace environment lacks the binary path. Evidence assertions remain unchanged.
