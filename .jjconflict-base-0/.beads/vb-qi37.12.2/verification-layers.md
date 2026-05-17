# Verification Layers - vb-qi37.12.2

STATUS: CONTRACT_NARROWED

## Boundary

- TLA+ temporal model: resume attempt workflow and append failure restoration.
- Rust/runtime shell: public error values, conversion/fallback behavior, storage journal failure propagation.
- API compatibility: public `ResumeError::JournalAppendFailed` unit variant remains semver-compatible.
- Static safety: no hidden ambient source side channel.
- Theorem projection: waived.

## Layer Assignment

- R1 -> focused integration tests + mutation + static scan.
- R2 -> focused integration tests + optional TLA+ `NoFalseResumedSuccess`.
- R3 -> focused integration tests + optional TLA+ `FailedAppendRestoresResumable`.
- R4 -> compile/static public type-shape check via focused tests or API checks.
- R5a/R5e -> API compatibility + focused tests for deterministic unit fallback.
- R5c/R5d/INV-003 -> static scan and tests proving no source detail is asserted unless the public error/API carries it; reject globals/thread-locals/task-locals/stale storage as source carriers.

## Commands/Evidence Boundaries

- Focused behavior command: `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation`
- API compatibility command: `cargo semver-checks check-release`
- Static scan command: `cargo clippy -p vb_runtime --lib --all-features -- -D warnings`
- Optional TLA+ command if model files are added: `tlc -config specs/vb_qi37_12_2_resume.cfg specs/vb_qi37_12_2_resume.tla`

## Waivers

- Lean/Aeneas/Hax waived: no theorem kernel required for a unit-variant representational limitation.
- Exact source preservation through `ResumeError::JournalAppendFailed` waived by contract narrowing. Exact source detail remains required only when carried by public error shape or approved explicit non-ambient API.
