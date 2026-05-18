bead_id: vb-v7x6
phase: 7
attempt: 1-of-7

- Run `cargo test -p xtask --test ui_release_gates -- --nocapture`.
- Run `cargo nextest run --cargo-quiet -p xtask --test ui_release_gates`.
- Run `moon run :doc` with external target to avoid tmpfs quota false negative.
- Run `moon ci` with external target as canonical gate.
