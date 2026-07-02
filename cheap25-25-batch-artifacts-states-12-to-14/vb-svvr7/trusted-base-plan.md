# Trusted Base Plan — vb-svvr7

## Bead: vb-svvr7 — IPC: reject trailing bytes in CLI postcard frame decoder (P1 bug)

Defines the trusted boundaries, assumptions, and trust markers for the proof effort. This is a minimal plan: the fix touches one crate (`vb_cli`), one module (`cli_postcard`), three files (`error.rs`, `validation.rs`, `tests.rs`), and one workspace-root proptest file (`verification/proptest/properties.rs`). There is one trust assumption, recorded as `TB-TB-01`.

---

## 1. Trusted Boundaries

### 1.1 vb_cli (Trusted Crate)

**Boundary**: `vb_cli` is the only crate that contains code edits. The fix is local to `cli_postcard::decode_postcard` (the function under repair) and `cli_postcard::PostcardError` (the enum that gains one variant). No other crate is touched.

**Trusted Components** (unchanged by the fix, used as the test substrate):

- `vb_cli::cli_postcard::PostcardHeader::from_bytes` (`crates/vb_cli/src/cli_postcard/types.rs:105-127`) — parses the 52-byte header.
- `vb_cli::cli_postcard::PostcardHeader::validate` (`crates/vb_cli/src/cli_postcard/types.rs:85-96`) — checks magic, header length, payload length bounds.
- `vb_cli::cli_postcard::encode_postcard` (`crates/vb_cli/src/cli_postcard/codec.rs:46-73`) — emits `HEADER_SIZE + p.len()` bytes exactly (regression-locked by `test_encode_postcard` and `test_roundtrip`).
- `vb_cli::cli_postcard::decode_postcard_json` (`crates/vb_cli/src/cli_postcard/codec.rs:24-34`) — public entry point; propagates `TrailingBytes` via `?` without remapping (CC-TB-6).
- `vb_cli::cli_postcard::payload_digest` (`crates/vb_cli/src/cli_postcard/validation.rs:22-27`) — BLAKE3 over payload bytes; used to validate `header.payload_digest`.
- `vb_cli::cli_postcard::validate_header_crc` (`crates/vb_cli/src/cli_postcard/validation.rs:29-43`) — CRC32 over header bytes 0..48.
- `vb_cli::cli_postcard::validate_version_and_kind` (`crates/vb_cli/src/cli_postcard/validation.rs:45-56`) — version bounds + kind match.
- `vb_cli::cli_postcard::validate_cli_payload` (`crates/vb_cli/src/cli_postcard/validation.rs:7-20`) — payload metadata validation (called by `decode_postcard_json`).
- `vb_cli::output::OutputError::PostcardFrame` (`crates/vb_cli/src/output.rs:138, 146`) — wraps `PostcardError` for CLI error reporting; consumes Display output, so the new Display arm is consumed without code change.
- `vb_cli::output::encode_postcard_json_frame` (`crates/vb_cli/src/output.rs:135-147`) — encoder path; uses the trusted `encode_postcard` above.

**Trust Rationale**: These are pure functions with no shared state, no I/O, no `unsafe` (workspace lint `unsafe_code = forbid`), and deterministic behavior. They form the trusted core of the CLI envelope codec.

### 1.2 vb_ipc (Trusted Crate, Sibling, Not Edited)

**Boundary**: `vb_ipc` is the IPC socket boundary. The sibling decoder `vb_ipc::frame::decode_frame_payload` (`crates/vb_ipc/src/frame.rs:35-51`) already uses `if payload.len() != expected_len` at line 44. The fix in `vb_cli` makes the two crate boundaries agree on the strict-length invariant (CC-TB-9). The sibling is read-only context, not edited.

**Trust Rationale**: `vb_ipc` shares the same workspace lint set (`unsafe_code = forbid`, `unwrap_used = forbid`, etc.) and is a verified sibling boundary. The parity lock is discharged by `cargo clippy` on `vb_cli` matching the sibling's single-compare pattern.

### 1.3 External Dependencies (Trusted at the Workspace Level)

| Component | Use | Trust Anchor |
|---|---|---|
| `postcard` 1.x | Encoder / inner decoder (`encode_postcard`, `decode_cli_payload`) | Workspace dependency (`Cargo.toml:39`); standard crate; serde-friendly |
| `serde`, `serde_json` 1.x | Postcard derives; JSON parse in `decode_postcard_json` | Workspace dependencies (`Cargo.toml:46-48`); standard crates |
| `serde-saphyr` | Not used in `cli_postcard`; declared in `crates/vb_cli/Cargo.toml:18` but not on the strict-length path | Trusted at the workspace level; not on the proof path |
| `crc32fast` 1.x | Header CRC validation | Workspace dependency (`Cargo.toml:28`); standard crate |
| `blake3` 1.x | Payload digest | Workspace dependency (`Cargo.toml:23`); standard crate |
| `chrono` | Not on the strict-length path; declared for date-time fields elsewhere in vb_cli | Trusted at the workspace level; not on the proof path |
| `proptest` 1.x | Property-based testing (in `verification/proptest/properties.rs` and as `vb_cli` dev-dep) | Workspace dev-dep (`Cargo.toml:40`); standard crate |
| `moon` 2.2.4 | Canonical CI gate (`moon run :source-lint`) | Installed at `/home/lewis/.local/share/mise/installs/npm-moonrepo-cli/2.2.4/bin/moon` |
| `rustc` + `clippy` nightly-2026-04-28 | Source lint + cargo clippy | Pinned in `rust-toolchain.toml` |

---

## 2. Trusted Assumptions

### 2.1 TB-TB-01 — proptest test-target wiring

**Assumption**: The proptest file at `verification/proptest/properties.rs` must be wired into a Cargo test target so that `cargo test -p vb_cli --test <target> -- strict_length_no_trailing_bytes` can run the new property `prop_strict_length_no_trailing_bytes`.

**Current state**: The proptest file is at the workspace root and is NOT picked up by `cargo test --workspace` because it is not under any crate's `tests/` directory. The file imports `vb_cli::cli_postcard::{...}` and uses `proptest::prelude::*`, so it needs to be compiled as a test binary inside the `vb_cli` crate.

**Required wiring (proof-writer responsibility)**:

Option A — add a `crates/vb_cli/tests/cli_postcard_properties.rs` file that includes the proptest file via `#[path = "../../../verification/proptest/properties.rs"] mod properties;` and then references `properties::prop_strict_length_no_trailing_bytes`. This is the minimal change.

Option B — move `verification/proptest/properties.rs` under `crates/vb_cli/src/verification/proptest/mod.rs` and re-export. This is more invasive but matches the `vb_runtime/src/verification/proptest/mod.rs` pattern observed in other beads.

The proof-writer picks one option; the obligation's `command` is `cargo test -p vb_cli --test cli_postcard_properties` either way. The TB-TB-01 row is the single `trusted_base_refs` entry on `PO-TB-PROP-01`.

**Compensating evidence if wiring fails**: the unit-test obligation `PO-TB-UNIT-01` covers the same property at the unit-test level (valid encode + 1 trailing byte → `Err(TrailingBytes)`). The unit test does not cover the `[1, 4096]` length range, but the bug-closure property is structurally the same: any extra byte after a valid frame must be rejected with `TrailingBytes`. So even if the proptest test target cannot be wired, the bug is closed by the unit test plus the cargo-clippy and source-lint obligations.

### 2.2 Workspace Lint Pinned

**Assumption**: `rust-toolchain.toml` pins `nightly-2026-04-28` (the version named in `.moon/tasks/all.yml:51` for `lint-src` and the rest of the CI gates). The `cargo clippy` and `moon run :source-lint` obligations both depend on this pin.

**Compensating evidence if the toolchain pin drifts**: the cargo-test obligation (`PO-TB-UNIT-01`) runs against any `cargo` and does not depend on the pin. The proptest obligation also runs against any `cargo`; the pin affects only the workspace lint set, not proptest.

### 2.3 PostcardError Trait Derives

**Assumption**: `PostcardError` uniformly derives `Debug, Clone, PartialEq, Eq` and implements `std::fmt::Display` and `std::error::Error`. The new `TrailingBytes` unit variant inherits these derives automatically. The unit test `test_postcard_error_variants` exercises `PartialEq` (via `assert_eq!` and `std::mem::discriminant`) and `Display` (via `format!`).

**Compensating evidence if a derive is missing**: the existing 11 variants all derive uniformly and the new variant is added in the same `enum` declaration; Rust's derive expansion is mechanical and any missing derive would surface as a compile error in `cargo check` (a precondition for the `cargo-test` and `cargo-clippy` obligations).

---

## 3. Stubs and Mocks

None. The fix is a one-line behavior change plus one enum variant plus one Display arm; no stubs, no mocks, no `#[cfg(...)]` gates, no `#[trusted]` annotations, no `#[verifier::external_body]`, no `#[verus::trusted]`, no `assume(`, no `axiom`, no `admit`, no `sorry`. The proptest uses `proptest::any::<Vec<u8>>()` which is a strategy generator, not a stub.

---

## 4. Model Reductions

| Reduction | Where | Sound? | Rationale |
|---|---|---|---|
| `proptest` `cases = 10000` | PO-TB-PROP-01 | yes | Standard proptest budget; the property is over a finite strategy space (`[0, MAX_PAYLOAD] x [1, 4096]`) and shrink reduces counterexamples to minimal shapes |
| `proptest` `input_size = 4096` | PO-TB-PROP-01 | yes | The contract requires arbitrary trailing length; 4096 is a generous upper bound; MAX_PAYLOAD is 65536, so the trailing space is much smaller than the payload space |
| `cargo clippy` `--all-targets` | PO-TB-CLIPPY-01 | yes | Standard cargo clippy surface; covers lib + bins + examples + tests + benches in vb_cli |
| `moon :lint-src` workspace lints | PO-TB-LINT-01 | yes | The CI gate runs against `--workspace --all-features` with the full lint set; matches the canonical gate per AGENTS.md |

No model reduction weakens the claim. The `cases = 10000` budget is the proptest default and matches `references/resource-governance.md`. The `input_size = 4096` upper bound is a property of the proptest strategy, not a soundness reduction.

---

## 5. Known Limitations

| Limitation | Impact | Mitigation |
|---|---|---|
| Proptest test target is not pre-wired in the workspace | PO-TB-PROP-01 cannot be executed until the proof-writer adds the wiring | TB-TB-01 records the assumption; PO-TB-UNIT-01 covers the same property at unit-test level as a compensating evidence |
| `verification/proptest/properties.rs` is at the workspace root, not under vb_cli | Same as above | Same as above |
| No fuzz target for `vb_cli::cli_postcard` | Hostile-input claim is satisfied by proptest, not by dynamic fuzz | Documented in VLD-TB-10 as `superseded_by_other_lane_with_evidence`; proptest over `[1, 4096]` arbitrary trailing bytes is the planned evidence |
| No Verus spec for `vb_cli::cli_postcard` | Refinement claim (if any) cannot be expressed | Documented in VLD-TB-05 as `surface_absent`; the property is a single integer compare and is fully discharged by proptest + cargo-test |
| `behavior_affecting: false` on all obligations | Bead is classified as a bug-fix lock-down, not a new feature | Per user policy "Behavior: false"; the contract_change is purely additive (CC-TB-9) and the bug-closure is a strict length invariant that the existing tests already cover at the boundary cases |

---

## 6. Evidence Commands for Trusted Base

```bash
# TB-TB-01: confirm the proptest file is at the expected path
test -f verification/proptest/properties.rs && echo 'TB-TB-01 file exists'

# TB-TB-01: confirm proptest file imports vb_cli
rg -n 'use vb_cli::cli_postcard' verification/proptest/properties.rs

# TB-TB-01: confirm proptest is in the workspace dev-deps
rg -n '^proptest' Cargo.toml

# TB-TB-01: confirm vb_cli dev-deps include proptest
rg -n '^proptest' crates/vb_cli/Cargo.toml

# TB-TB-01 (Option A wiring): add a tests/ file under vb_cli
ls -la crates/vb_cli/tests/ 2>/dev/null || echo 'tests/ does not exist yet'

# Workspace lint pin
cat rust-toolchain.toml
```

The proof-writer MUST run the cargo-test obligation `PO-TB-UNIT-01` and the cargo-clippy obligation `PO-TB-CLIPPY-01` before the proptest obligation `PO-TB-PROP-01`, so that any compile error in the new code is caught by the lower-cost gates first. The source-lint obligation `PO-TB-LINT-01` is the canonical final gate.
