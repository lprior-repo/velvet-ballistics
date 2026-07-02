# Hazard Analysis — vb-tsjnz

- bead_id: `vb-tsjnz`
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`
- capture: 2026-07-01

## Hazard List

### H-LINT-FORWARD — Lint Policy Forward-Applies to Existing Source (PRIMARY HAZARD)

**Category:** Rust-core invariant (`unwrap_used`, `expect_used`, `panic`, etc. forbid lints).

**Risk:** Adding `[lints]\nworkspace = true` to `crates/vb_queue_semantics/Cargo.toml` flips the build acceptance surface for `crates/vb_queue_semantics/src/lib.rs` from `#![deny(unused_must_use, unreachable_pub, rust_2018_idioms)] + #![forbid(unsafe_code)]` headers only, to the **full workspace deny set**:

- `unsafe_code = "forbid"` — already covered by source-level attribute.
- `unused_must_use = "deny"` — already covered.
- `unreachable_pub = "deny"` — already covered.
- `rust_2018_idioms = "deny"` — already covered.
- `unexpected_cfgs = "warn"` — already covered.
- `correctness / suspicious / perf / complexity = "deny"` — new.
- `unwrap_used / expect_used / panic / panic_in_result_fn / todo / unimplemented / dbg_macro = "forbid"` — **new and decisive**.
- `indexing_slicing / arithmetic_side_effects / as_conversions / let_underscore_must_use / await_holding_lock / get_unwrap = "deny"` — new.
- `string_slice = "forbid"` — new.
- `large_stack_arrays / large_types_passed_by_value / result_large_err = "warn"` — new.

Scout report (referenced by `codebase-map.md`) indicates no `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!` was found in `lib.rs` via Grep. The scout flagged this as **UNCONFIRMED**. Holzman-rust MUST re-verify before claiming green. **Catastrophe mode:** if a deniable pattern does exist, the patch fails compile; the failure cannot be silenced by relaxing the workspace lints (rule: no loop oscillations).

**Mitigation:**

- Holzman-rust runs `cargo check -p vb_queue_semantics --all-targets` and `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` immediately after the edit.
- On failure, the bead reports `LintFailure` and hands off the source fix to the next bead — refusing to alter the workspace lints to make the build pass.
- The clippy deny set is large; `-D warnings` makes the warnings fatal, so all of `large_stack_arrays` etc. also block.

**Likelihood:** Low (scout negative); **Impact:** Build-blocking; **Risk score:** Medium (because the scout negative is grep-based, not build-based).

### H-CARGO-METADATA — Workspace Inheritance Resolution Failure

**Category:** Cargo-metadata resolution.

**Risk:** Replacing the literal `version` with `version.workspace = true` requires `[workspace.package].version` to exist. It does (workspace root line 19, value `"0.1.0"`). If a future change to the workspace root removes that key, the patch becomes invalid. Out of scope to guard against in this bead.

**Mitigation:** none required pre-merge; the dependency on the workspace key is part of the design (the whole point).

**Likelihood:** Trivial if not touched; **Impact:** Build-blocking; **Risk score:** Low (the workspace root is canonical and held by the workspace author).

### H-MANIFEST-DRIFT — Reintroduction of Literal `version`

**Category:** Release/API governance drift.

**Risk:** A future developer (or a sloppy merge) reintroduces `version = "..."` in `crates/vb_queue_semantics/Cargo.toml`. The drift does not surface immediately because cargo accepts both forms; it is a long-tail governance loss.

**Mitigation:** black-hat reviewer audits landing; consider a future cargo-meta CI check (out of scope).

**Likelihood:** Low; **Impact:** Documentation/governance; **Risk score:** Low.

### H-LINT-AXIS — `lints.rust` vs `lints.clippy` Boundary

**Category:** Cargo `[lints]` semantics.

**Risk:** `[lints]\nworkspace = true` pulls BOTH `[workspace.lints.rust]` AND `[workspace.lints.clippy]` en bloc. There is no per-axis opt-out in Cargo's design. A reviewer who expected selective inheritance would have to re-architect.

**Mitigation:** Documented in `domain-model.md` and `error-taxonomy.md`. Sibling crates use the same en-bloc opt-in. Any future migration to per-axis lints is a workspace-wide change requiring its own bead.

**Likelihood:** Trivial; **Impact:** Documentation; **Risk score:** Low.

### H-CARGO-FEATURE — Older Cargo Lack of `[lints]` Support

**Category:** Toolchain compatibility.

**Risk:** Cargo's `[lints]` table requires Cargo 1.74+ (stabilized 2023-11-16). `rust-toolchain.toml` pins a newer nightly per `rust-governance.md`. If the pin decays, the lint block becomes a syntax error.

**Mitigation:** Trust the rust-toolchain pin; verify post-build with a successful `cargo check`. The patch itself does not modify `rust-toolchain.toml`.

**Likelihood:** Trivial; **Impact:** Build-blocking; **Risk score:** Low.

### H-PUBLIC-API — Semver Drift from `version.workspace = true`

**Category:** Release/API surface.

**Risk:** Workspace version is `"0.1.0"`. Current literal is also `"0.1.0"`. The numerical version of the published crate is unchanged. **No public-API drift.**

**Mitigation:** Confirmed by scout. Documented as no risk.

**Likelihood:** Zero; **Impact:** N/A; **Risk score:** None.

### H-CI-LOOP — `.moon/tasks/all.yml` Coverage Delta

**Category:** CI loop coverage.

**Risk:** `.moon/tasks/all.yml:396,399` iterates over workspace members for `cargo machete` and supply-chain scans; `vb_queue_semantics` was already in the loop. Adding `[lints]\nworkspace = true` does not change coverage. **No delta.**

**Mitigation:** None required.

**Likelihood:** Zero; **Impact:** N/A; **Risk score:** None.

### H-OUT-OF-SCOPE-BLEED — Touching `lib.rs` or `vb-2lu1`

**Category:** Process hazard.

**Risk:** A sloppy Edit mangles `crates/vb_queue_semantics/src/lib.rs` (e.g. accidental whitespace, accidental comment), or removes the `vb-2lu1` exception from `.config/source-length-exceptions.txt:323`. The former adds lint risk; the latter triggers a 300-line cap on a 423-line file.

**Mitigation:** The Edit MUST be confined to lines 3 (replace) and 11→13 (append). The implementer performs a `jj diff` before committing and a `cargo check` afterward. Any `lib.rs` change is a `Failed::OutOfScopeBleed`.

**Likelihood:** Low; **Impact:** Build- or policy-blocking; **Risk score:** Medium (process-only).

### H-XTASK-LIST — `xtask/src/forbidden_scan.rs` Coverage

**Category:** xtask allow-list drift.

**Risk:** `xtask/src/forbidden_scan.rs:18` lists `vb_queue_semantics` in an allow-list. Cargo.toml patch does not move the crate. No drift.

**Mitigation:** None required.

**Likelihood:** Zero; **Impact:** N/A; **Risk score:** None.

### H-VERUS-HELPER — `generate_queue_state_verus_helpers.py` Coverage

**Category:** Verus helper generator.

**Risk:** `scripts/generate_queue_state_verus_helpers.py:4,19` reads `crates/vb_queue_semantics/src/lib.rs`. The patch does not touch the file; the generator's output is unaffected. **No drift.**

**Mitigation:** None required.

**Likelihood:** Zero; **Impact:** N/A; **Risk score:** None.

### H-CONCURRENCY / H-UNSAFE / H-FFI / H-NETWORK / H-PARSER / H-PERFORMANCE

**Category:** Not applicable.

**Risk:** The bead does not touch any code that introduces concurrency, unsafe, FFI, network I/O, parsing, or performance-sensitive paths. **No risk introduced.**

**Mitigation:** N/A.

## Hazard Summary Table

| Hazard | Category | Likelihood | Impact | Risk |
| --- | --- | --- | --- | --- |
| H-LINT-FORWARD | Rust-core invariant | Low | Build-blocking | Medium |
| H-CARGO-METADATA | Cargo-metadata | Trivial | Build-blocking | Low |
| H-MANIFEST-DRIFT | Governance | Low | Doc loss | Low |
| H-LINT-AXIS | Cargo `[lints]` semantics | Trivial | Doc only | Low |
| H-CARGO-FEATURE | Toolchain compatibility | Trivial | Build-blocking | Low |
| H-PUBLIC-API | Release/API | Zero | N/A | None |
| H-CI-LOOP | CI loop coverage | Zero | N/A | None |
| H-OUT-OF-SCOPE-BLEED | Process | Low | Build/policy-blocking | Medium |
| H-XTASK-LIST | xtask list drift | Zero | N/A | None |
| H-VERUS-HELPER | Verus generator | Zero | N/A | None |
| H-CONCURRENCY/UNSAFE/FFI/NETWORK/PARSER/PERFORMANCE | not applicable | Zero | N/A | None |

## H-LINT-FORWARD Mitigation Plan (Primary Mitigation)

1. Pre-edit: scout reports grep result.
2. Post-edit: holzman-rust runs `cargo check -p vb_queue_semantics --all-targets`.
3. Post-edit: holzman-rust runs `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings`.
4. Either step non-zero → report `Failed::LintFailure`.
5. Forbidden recovery moves: altering workspace lint policy, adding `#[allow(...)]` to `vb_queue_semantics` source, lowering priority at workspace root — all prohibited under Holzman-Rust doctrine.

## Out-of-Scope Hazard Recording

- A genuine `unwrap()`/`expect()`/`panic!()` in `lib.rs` is the next-bead hazard; this bead will surface it but not fix it. The fix is **clearly the responsibility of a follow-up source-cleanup bead**.
- A future workspace version bump is a release-coordination hazard owned by `release-eng` (or analogous), not this bead.
