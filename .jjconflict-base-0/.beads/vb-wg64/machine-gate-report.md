# vb-wg64 Machine Gate Report

## Required Focused Gates

- `rtk cargo fmt --all -- --check`: PASS, exit 0. Log: `/tmp/vb-wg64-fmt.log`.
- `rtk cargo clippy -p xtask --all-targets -- -D warnings`: PASS, exit 0. Log: `/tmp/vb-wg64-clippy-xtask.log`.
- `rtk cargo clippy -p vb_cli --all-targets -- -D warnings`: PASS, exit 0. Log: `/tmp/vb-wg64-clippy-vb-cli.log`.
- `rtk cargo check -p vb_storage --test recovery_bdd_tests`: PASS, exit 0. Log: `/tmp/vb-wg64-storage-check.log`.

## Additional Focused Gates

- `rtk cargo check -p velvet-ballistics-workspace-tests`: PASS, exit 0.
- `rtk cargo build --manifest-path fuzz/Cargo.toml --bins`: PASS, exit 0.
- `rustup run nightly-2026-04-28 cargo check --quiet --manifest-path crates/vb_ui_model/Cargo.toml --no-default-features`: PASS, exit 0.
- `rustup run nightly-2026-04-28 cargo bench --quiet -p velvet-ballistics-workspace-tests --bench velvet_ballistics --bench vb_qi37_1_1_recovery --all-features --no-run`: PASS, exit 0.

## Final Gate

- `moon ci --base HEAD --head HEAD --force`: PASS, exit 0.
- Final log: `/tmp/vb-wg64-moon-ci-final.log`.
