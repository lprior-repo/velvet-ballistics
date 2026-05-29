# TOOLING-FLUX Flux Setup Audit

State: setup-audit
Sublane: flux-setup
Attempt: 1
Workdir: `/home/lewis/src/velvet-ballistics`

## Verdict

Flux tooling is installed, but repository Flux lanes are not consistently wired to real crate-mode verification.

No production-semantic patch was applied. Cargo metadata/dependency changes are not low-risk because they would expand checked scope and prior `vb-rpch` evidence records a full-crate `vb_storage` Flux internal compiler error when `[package.metadata.flux] enabled = true` was temporarily probed.

## Tool Discovery Evidence

| Command | Result |
| --- | --- |
| `rustup show active-toolchain` | `nightly-2026-04-28-x86_64-unknown-linux-gnu`, overridden by repo `rust-toolchain.toml` |
| `cargo flux --help` | PASS; accepted package/workspace/feature flags; no `--lib` or `--test` target flags listed |
| `cargo flux -V` | `cargo-flux 4d329f2 (2026-05-23)` |
| `cargo flux --version` | `cargo-flux 4d329f2 (2026-05-23)` |
| `z3 --version` | `Z3 version 4.16.0 - 64 bit` |
| `fixpoint --version` | `fixpoint 0.9.6.3.6 (6f214fd7a67c1e61f3f165569b88dfdec2dda0d9)` |
| `liquid-fixpoint --version` | `fixpoint 0.9.6.3.6 (6f214fd7a67c1e61f3f165569b88dfdec2dda0d9)` |
| `bd show TOOLING-FLUX` | FAIL; no issue found matching `TOOLING-FLUX` |

## Command Convention Evidence

| Command | Result |
| --- | --- |
| `cargo flux -p vb_core --lib` | FAIL; `error: unexpected argument '--lib' found` |
| `cargo flux -p vb_core --test anything` | FAIL; `error: unexpected argument '--test' found` |
| `cargo flux -p vb_storage --message-format human -j 1` | PASS; `Finished flux profile ... in 9.57s` |

The installed `cargo-flux` supports `cargo flux -p <package>` and `cargo flux --package <package>`. It does not support Cargo target selectors such as `--lib` or `--test`.

## Repository Setup Findings

1. `verification/flux/vb_xi2f_compile_source.rs`, `verification/flux/vb_xi2f_try_from_parts.rs`, and `verification/flux/choose_refinements.flux` claim `cargo flux --package vb_compile` / `vb_core`, but they are standalone artifacts outside those crate source trees.
2. `crates/vb_core/Cargo.toml`, `crates/vb_compile/Cargo.toml`, and `crates/vb_storage/Cargo.toml` do not declare `flux-rs` dependencies or `[package.metadata.flux]`.
3. `cargo flux -p vb_storage --message-format human -j 1` passing is only a crate driver smoke pass. It is not proof that detached `verification/flux/*.rs` artifacts were checked.
4. Trusted/ignore scan over `verification/flux` found no `#[trusted]`, `#[trusted_impl]`, `#[extern_spec]`, or `#[ignore]`; only `#![forbid(unsafe_code)]` markers were present.
5. `.beads/vb-rpch/trusted-base-ledger.flux-r9.jsonl` records `BLOCKED_TOOLING_FOR_FULL_CRATE_VERIFICATION` after a temporary metadata probe hit a Flux internal compiler error in `crates/vb_storage/src/admission.rs:270`.

## Root Causes

1. Some Flux lanes treat package-level smoke commands as if they check standalone `verification/flux` artifacts.
2. Planned Flux commands do not encode whether the intended mode is crate-mode (`cargo flux -p <package>`) or single-file mode (`flux --crate-type=lib <file>`).
3. `cargo flux --lib` and `cargo flux --test` are invalid for the installed `cargo-flux`; future obligations using those flags will reject before verification starts.
4. Enabling crate-mode Flux broadly is not low-risk in this repo because prior evidence shows `vb_storage` full-crate Flux checking can ICE when metadata is enabled.

## Recommended Follow-Up Beads

1. Add a Flux command-convention bead: normalize obligations to `cargo flux -p <package> --message-format human` for crate smoke, and forbid `--lib` / `--test` in Flux lanes unless upstream help changes.
2. Add a detached-artifact wiring bead: either move selected Flux artifacts into crate-owned checked modules behind approved features, or create a documented single-file `flux --crate-type=lib <file>` lane with exact dependency assumptions.
3. Add a Flux metadata adoption bead per crate: start with the smallest package, add `[package.metadata.flux]` and `flux-rs` only after an approved scope plan, and record ICE/tool blockers separately from proof failures.
4. Add a proof-obligation hygiene bead: reject Flux proof closure when the checked command does not include or compile the named `verification/flux` artifact.
