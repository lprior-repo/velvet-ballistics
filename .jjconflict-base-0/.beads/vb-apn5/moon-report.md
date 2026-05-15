bead_id: vb-apn5
bead_title: "storage/runtime: Single-server database lock enforcement"
phase: 8
updated_at: 2026-05-09T00:00:00Z

# Machine Gate Report

## Gates Executed

| Gate | Command | Status | Evidence |
|---|---|---|---|
| :quick | `moon run :quick` | PASS | fmt, lint-src, check, nightly-feature-gate green |
| :check | `moon run :check` | PASS | Compiled successfully |
| :lint-src | `moon run :lint-src` | PASS | Zero clippy warnings for changed code |
| vb_storage + vb_runtime | `cargo nextest run -p vb_storage -p vb_runtime --all-features` | PASS | 2090 passed, 0 failed |
| vb_storage suite | `cargo test -p vb_storage` | PASS | 776 passed, 0 failed |
| vb_runtime suite | `cargo test -p vb_runtime` | PASS | 1314 passed, 0 failed |

## CI Failure Classification
- Category: N/A — all gates green

STATUS: GREEN
