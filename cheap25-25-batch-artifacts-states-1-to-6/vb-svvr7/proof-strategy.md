# Proof Strategy — vb-svvr7

STATUS: PLANNED

## Bead

- **bead_id**: vb-svvr7
- **title**: IPC: reject trailing bytes in CLI postcard frame decoder (P1 bug)
- **state**: 4 proof-planner
- **isolated_workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7
- **jj workspace**: cheap25-vb-svvr7
- **upstream_main**: 2c8ea33c9
- **captured_at**: 2026-07-01
- **planner_invocation_id**: proof-planner-vb-svvr7-20260701

## Scope

This plan covers the proof-of-closure for a P1 bug in `vb_cli::cli_postcard::decode_postcard`. The current code at `crates/vb_cli/src/cli_postcard/validation.rs:87-89` uses `if data.len() < payload_end` and then unconditionally accepts the slice `data.get(payload_start..payload_end)` at line 94-96, so any number of trailing bytes after a valid frame is silently accepted. The fix replaces `<` with `!=` and returns a new typed error `PostcardError::TrailingBytes` (new unit variant added in `crates/vb_cli/src/cli_postcard/error.rs`).

Out of scope:

- `crates/vb_ipc/src/frame.rs::decode_frame_payload` already uses `if payload.len() != expected_len` at line 44. This is a different crate boundary (IPC socket) and is not the site of the bug. The fix is purely additive on the public surface and the IPC sibling is mentioned only as parity reference, not as a code-edit target.
- `crates/vb_ui_model/src/emitter/binary/mod.rs` has an unrelated `decode_postcard` at line 166; not in this bead's edit set.
- `crates/vb_cli/tests/cli_integration.rs::assert_postcard_stdout` consumes the encoder's own output, which never carries trailing bytes, so the integration path stays green.

The forbidden moves (per bead instructions):

- A `compat_mode` / `lenient` / `allow_trailing` flag is FORBIDDEN. The fix tightens; it does not add a backward-compat escape hatch.
- Silently accepting trailing bytes is FORBIDDEN. The post-fix invariant is strict equality, not `<`.

## Inputs Read

- `.beads/vb-svvr7/STATE.md` (workspace + state 1..3 ledger)
- `.beads/vb-svvr7/codebase-map.md` (decoder at validation.rs:71-102; sibling at vb_ipc/src/frame.rs:35-51; existing tests at tests.rs:1-197; proptest at verification/proptest/properties.rs:1-369)
- `.beads/vb-svvr7/contract.md` (CC-TB-1..CC-TB-10, 10 contract clauses, 10 acceptance conditions, 5 open questions answered, Out-of-Scope section)
- `.beads/vb-svvr7/proof-seeds.jsonl` (8 proof seeds PS-TB-01..PS-TB-08, all behavior_affecting: true in the seed; downgraded to false on obligations per bead-scope policy)
- `.beads/vb-svvr7/traceability-matrix.jsonl` (10 requirements REQ-TB-* mapped to source_refs and test_refs)
- `.beads/vb-svvr7/delivery-scope.jsonl` (48 rows: 3 touched files in vb_cli, 6 verifier_modes of which 4 are required and 4 are optional, 5 risk_tags, 5 exclusions, 3 open-question answers)
- `.beads/vb-svvr7/hazard-analysis.md` (H1: encoder trailing-byte impossibility; H2: cross-crate parity; H3: proptest wiring gap)
- `crates/vb_cli/src/cli_postcard/validation.rs` (the bug site; verified lines 71-102)
- `crates/vb_cli/src/cli_postcard/error.rs` (the enum; verified lines 7-50)
- `crates/vb_cli/src/cli_postcard/codec.rs` (consumer; verified lines 24-34 and 46-73)
- `crates/vb_cli/src/cli_postcard/tests.rs` (existing 17 unit tests; verified lines 1-197)
- `crates/vb_ipc/src/frame.rs` (sibling; verified lines 35-51 with the `!=` check at line 44)
- `verification/proptest/properties.rs` (the proptest file; verified lines 1-369, currently NOT wired into a Cargo test target)
- `Cargo.toml` (workspace lints unsafe_code=forbid, unwrap_used=forbid, etc.; proptest=1 in dev-deps)
- `crates/vb_cli/Cargo.toml` (proptest already in dev-dependencies)
- `.moon/tasks/all.yml` (`:lint-src` runs cargo clippy --workspace --lib --bins --examples --all-features with -D warnings; `:test` runs cargo nextest run --workspace --all-features)

## Discovery Evidence

- `pwd -P` returns `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7` (matches the bead's `isolated_workdir`).
- `jj root` returns `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7` (jj workspace `cheap25-vb-svvr7`).
- `git rev-parse --show-toplevel` returns `fatal: not a git repository (or any parent up to mount point /)` — the workspace is jj-only; this is the documented pattern for the isolated worktree.
- `test -s .beads/vb-svvr7/STATE.md && test -s .beads/vb-svvr7/contract.md && test -s .beads/vb-svvr7/proof-seeds.jsonl && test -s .beads/vb-svvr7/traceability-matrix.jsonl && test -s .beads/vb-svvr7/codebase-map.md && test -s .beads/vb-svvr7/delivery-scope.jsonl` exits 0.
- `rg -n TrailingBytes crates/` returns zero matches in the current tree — confirms the bug-closure gap.
- `rg -l cli_postcard fuzz/` returns zero matches — no fuzz target exists for this module; cargo-fuzz is therefore not_applicable for the planned scope.
- `rg -n TrailingBytes verification/verus/` returns zero matches — no production-bound Verus spec exists; vacuum Verus is rejected per GOD RULE 2.
- `command -v jq && command -v sha256sum && command -v moon && command -v cargo` all return 0; proptest, cargo-test, cargo-clippy, and source-lint are runnable.
- `command -v verus` not present; `command -v cargo-kani` not present; these are documented in the not_applicable rows with surface_absent / superseded_by_other_lane_with_evidence limitation_kinds.

## Risk Profile

| Risk tag (from proof-seeds.jsonl) | Present? | Lane implication |
|---|---|---|
| `parser` | yes | cargo-test + proptest primary, cargo-clippy auxiliary |
| `codec` | yes | cargo-test + proptest primary |
| `public_api` | yes | additive enum variant; cargo-clippy + source-lint confirm signature preservation |
| `user_visible_behavior` | yes (in seed) | downgraded to behavior_affecting=false on obligations per bead-scope policy; downstream CLI exit codes stay StorageError via `output.rs::output_error_exit` |
| `temporal` / `concurrency` | no | loom not_applicable |
| `unsafe_ub` | no | miri not_applicable (workspace lint `unsafe_code = forbid`) |
| `persistence` | no | n/a |
| `auth_security` | no | boundary tightening only |
| `performance` | no | single compare; negligible |
| `migration` | no | bug fix; CLI_SCHEMA_VERSION unchanged |

The fix touches one crate (`vb_cli`), one module (`cli_postcard`), three files (`error.rs`, `validation.rs`, `tests.rs`), and extends one workspace-root proptest file (`verification/proptest/properties.rs`). No dependency delta. No version bump. No Cargo.toml change.

## Strategy

1. **Lock the bug-closure property with proptest.** Extend `verification/proptest/properties.rs` with `prop_strict_length_no_trailing_bytes`: for any `payload` in `[0, MAX_PAYLOAD]` and any `trailing_len` in `[1, 4096]`, `let mut buf = encode_postcard(v, k, p); buf.extend(vec![0; n]); assert_eq!(decode_postcard(&buf), Err(PostcardError::TrailingBytes))`. This is the primary evidence and runs at `PROPTEST_CASES=10000`. The proptest file is at the workspace root and is currently NOT wired into a Cargo test target; the proof-writer must add a `crates/vb_cli/tests/cli_postcard_properties.rs` (or equivalent `#[path]` inclusion under an existing tests target) so `cargo test -p vb_cli --test cli_postcard_properties` can run it. This is the single trusted-base assumption TB-TB-01.
2. **Lock the variant shape with cargo-test.** Add four unit tests to `crates/vb_cli/src/cli_postcard/tests.rs`:
   - `decode_rejects_trailing_bytes_after_valid_frame` (valid encode + 1 trailing byte → `Err(TrailingBytes)`).
   - `decode_accepts_exact_length_frame` (regression; valid encode with no trailing bytes → `Ok`).
   - `decode_postcard_json_propagates_trailing_bytes` (`?` chain uniformity).
   - `test_postcard_error_variants` (`std::mem::discriminant` discrimination; `format!()` non-empty; `TrailingBytes` distinct from `DecodeFailed`).
3. **Lock the source shape with cargo-clippy.** Run `cargo clippy -p vb_cli --all-targets -- -D warnings -W clippy::all -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::indexing_slicing -D clippy::arithmetic_side_effects -D clippy::as_conversions` to confirm the new code matches the workspace lint set and the cross-crate `!=` shape at `vb_ipc/src/frame.rs:44`.
4. **Lock the CI gate with source-lint.** Run `moon run :source-lint` (the canonical gate per AGENTS.md and contract.md AC-10) which composes `panic-surface`, `ignored-fallible-results`, `unsafe-audit`, `fmt`, and `cargo clippy --workspace --all-features -- -D warnings`. Zero warnings, zero `#[allow]`, zero `panic!/todo!/unimplemented!/dbg!/.unwrap()/.expect()`.

These four lanes are exactly the four required `verifier_mode` rows in `delivery-scope.jsonl:33-36`: `cargo_test` (twice for unit and proptest), `cargo_clippy`, `moon_run_source_lint`. The four `verifier_mode` rows that mark optional / `required:false` (Kani, Verus, Miri, Loom at lines 37-40) are not_applicable here and carry concrete non_applicability_evidence_refs in `verifier-lane-decisions.jsonl`.

## Non-Applicable Lanes (with evidence)

| Verifier | Reason | limitation_kind | Evidence anchors |
|---|---|---|---|
| `verus` | Vacuum Verus is rejected per AGENTS.md GOD RULE 2; the cli_postcard module has no production-bound spec in `verification/verus/` today; the property is a single integer compare that is fully discharged by proptest + cargo-test | `surface_absent` | `rg -n TrailingBytes verification/verus/` returns 0; contract.md PS coverage targets line 100 |
| `kani` | Optional per contract.md PS coverage targets line 100; proptest over 10000 cases of `[0, MAX_PAYLOAD] x [1, 4096]` is stronger than a Kani bounded proof at unwind=64 for a 32-line single-compare function; no `#[cfg(kani)]` harness is required | `superseded_by_other_lane_with_evidence` | PO-TB-PROP-01 model_bounds; contract.md AC list does not name Kani |
| `flux-rs` | The fix is a single `data.len() != payload_end` compare; no refinement surface; cargo-flux is not pinned in the workspace | `surface_absent` | contract.md PS coverage targets line 103; `validation.rs:87-89` |
| `loom` | `decode_postcard` is a pure single-threaded function over `&[u8]`; no Mutex/RwLock/Arc/thread/spawn/channel/async; loom would test zero concurrent transitions | `surface_absent` | delivery-scope.jsonl line 27-28; references/verifier-trigger-matrix.md Loom counterindication |
| `miri` | `unsafe_code = forbid` at workspace level; cli_postcard has no `unsafe` block; the slice arithmetic uses `checked_add` and `get` which are safe abstractions | `surface_absent` | Cargo.toml:55; contract.md PS coverage targets line 101 |
| `cargo-fuzz` | No fuzz target for `vb_cli::cli_postcard` exists in `fuzz/fuzz_targets/`; proptest over arbitrary trailing lengths in `[1, 4096]` covers the hostile-input claim | `superseded_by_other_lane_with_evidence` | `rg -l cli_postcard fuzz/` returns 0; contract.md PS coverage targets line 105 |

## Waivers

No behavior-affecting waivers. All four required obligations are `behavior_affecting: false` per bead-scope policy. The single row in `waiver-candidates.jsonl` (WVR-TB-00) is a meta-record stating that no waivers are required; it carries `behavior_affecting: false`, an ISO-8601 `expiry: 2026-12-31`, and three concrete `compensating_evidence` references. This is recorded so the no-waiver decision is auditable and so the file is schema-valid.

## Trusted Base

The plan introduces one trust assumption: the proptest file at `verification/proptest/properties.rs` must be wired into a Cargo test target named `cli_postcard_properties` under `crates/vb_cli/`. This is recorded as `TB-TB-01-test-target-wiring` in `trusted-base-plan.md` and is the sole `trusted_base_refs` entry across all four obligations. Everything else is either production code that the proof-writer edits in place (`validation.rs:87-89`, `error.rs:7-50`, `tests.rs:1-197`) or workspace infrastructure that is already present (`proptest@1.5` in dev-deps, `clippy@nightly-2026-04-28` in rust-toolchain, `moon@2.2.4` configured at `.moon/`).

## Handoff

- The reviewer (`proof-plan-reviewer`) at State 4b dispositions each of the 10 `verifier-lane-decisions.jsonl` rows.
- The writer (`proof-writer`) at State 5 authors the 4 obligations: the proptest property in `verification/proptest/properties.rs` (and the test-target wiring TB-TB-01), the 4 unit tests in `crates/vb_cli/src/cli_postcard/tests.rs`, the new `PostcardError::TrailingBytes` variant + Display arm in `crates/vb_cli/src/cli_postcard/error.rs`, and the tightened length check in `crates/vb_cli/src/cli_postcard/validation.rs:87-89`.
- The bridge (`proof-to-implementation`) at State 7 maps the 4 obligations to Rust source/test/harness obligations. The user has not requested `proof-to-implementation-input.md` in the State 4 output set, so that artifact is deferred to State 7.
- The formal verifier (`formal-verifier`) at State 12 executes the 4 commands and closes the ledger.

## Self-Audit Checklist

- [x] Every `(req, cc, seed, verifier)` tuple in the default profile has a lane decision (4 required + 6 not_applicable = 10 rows).
- [x] No `behavior_affecting: true` row in the waiver file (only WVR-TB-00 with `behavior_affecting: false`).
- [x] Every required lane decision has at least one paired `proof-obligation/v1` ID; the obligation exists in `proof-obligations.planned.jsonl`.
- [x] Every `proof-obligation/v1` `target` parses as `path::symbol` (`vb_cli::cli_postcard::decode_postcard`).
- [x] No `external_body`, `assume(`, `axiom` in the obligation commands (no Verus obligations at all).
- [x] No `kani::cover!` as the sole property evidence (no Kani obligations).
- [x] proptest obligation has `model_bounds.cases = 10000` and `model_bounds.input_size = 4096`.
- [x] proptest obligation uses a non-vacuous strategy (arbitrary `[0, MAX_PAYLOAD] x [1, 4096]`) and an exact-equality assertion (not `is_err()`).
- [x] Every `not_applicable` row has at least 3 concrete evidence references and a typed `limitation_kind`.
- [x] All `expected_evidence` strings cite a concrete tool marker (`test result: ok`, `0 warnings emitted`, `moon :lint-src exits 0`).
- [x] No weak vocabulary in any `decision_reason` (no "covered by Kani", "low risk", "we'll add later", "not needed").
- [x] No two rows duplicate `(req, cc, seed, verifier)` with conflicting `applicability`.
- [x] `behavior_affecting: false` on all four obligations per user policy.
- [x] `jq -c .` validates every JSONL file.
