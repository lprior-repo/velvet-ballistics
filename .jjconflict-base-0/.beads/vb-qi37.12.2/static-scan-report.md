# Static Scan Report — vb-qi37.12.2 State 11 Rerun

STATUS: PASS

- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo fmt --check`: PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo clippy -p vb_runtime --lib --tests --all-features -- -D warnings`: PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings`: PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo check -p vb_ipc --all-features`: PASS.

No warnings promoted to errors. No static-scan blocker remains.
