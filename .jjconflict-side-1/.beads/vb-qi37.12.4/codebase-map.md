bead_id: vb-qi37.12.4
bead_title: quality: Gate ignored fallible results
phase: 2
updated_at: 2026-05-15T00:00:00Z
attempt: 1-of-7

# Codebase map: ignored fallible-result gate

## Scope summary

This bead is a quality-gate bead. It should add a reproducible mechanical gate, not repair runtime/storage behavior directly. Acceptance requires the gate to fail when first-party production code silently discards fallible outcomes or documents a silent discard as intentional without explicit non-production/justified status.

Source checkout is `/home/lewis/src/velvet-ballistics`; isolated workspace is `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4`.

## Bead facts

- `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.12.4 --json` returned status `in_progress`, priority `0`, title `quality: Gate ignored fallible results`.
- Direct dependencies observed from bead JSON:
  - `vb-qi37.12.1` closed: audit silent discard sites.
  - `vb-qi37.12.2` blocked: propagate journal/storage failures.
  - `vb-qi37.12.3` closed: preserve action/recovery errors.
- Parent dependent `vb-qi37.12` remains `in_progress` and requires no first-party runtime/storage/compiler path silently drops fallible outcomes.

## Existing policy and gate surfaces

### Workspace lint contract

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/Cargo.toml`
  - Lines 136-160 define workspace lints.
  - `unused_must_use = "deny"` already exists.
  - `clippy::let_underscore_must_use = "deny"` already exists.
  - First-party crates opt into workspace lints through `[lints] workspace = true` in their Cargo.toml files.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/velvet-ballistics-MASTER.md`
  - Lines 1396-1420 describe the required lint contract, including ignored `Result` and `let_underscore_must_use` policy.
  - Lines 1707-1720 require CI gate coverage and hard clippy flags, including `-D clippy::let_underscore_must_use`.

### Moon gate surface

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/.moon/tasks/all.yml`
  - `lint-src` at lines 42-53 runs `cargo clippy --workspace --lib --bins --examples --all-features` with hard denies, but currently does not include `-D clippy::let_underscore_must_use`, `-D clippy::panic_in_result_fn`, `-D clippy::indexing_slicing`, `-D clippy::string_slice`, `-D clippy::get_unwrap`, `-D clippy::arithmetic_side_effects`, or `-D clippy::as_conversions` on the command line even though workspace lints cover many crates.
  - `check` at lines 55-71 runs `cargo check --workspace --all-targets --all-features` with `RUSTFLAGS=-Dwarnings` and depends on feature, agent CLI, and beads server mode checks.
  - `verify-standard` at lines 480-491 runs `scripts/rust-verification-gauntlet.sh standard`; this is the canonical later-state verifier mode unless deeper lanes are required by contract.

### xtask gate surface

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/xtask/src/gates.rs`
  - `Gate::Clippy` command is `cargo +nightly clippy --workspace` only, so command-center clippy evidence may be weaker than Moon `lint-src` and weaker than master lint expectations.
  - `Gate::ForbiddenScan` and `Gate::HotpathScan` point to `scripts/forbidden-scan.sh` and `scripts/hotpath-scan.sh`, but those scripts were not present in the workspace glob results; treat this as a stale command-center risk unless another state proves they exist or are generated.
  - Existing `Gate::SourceLength` wraps `scripts/check-source-length.sh`; a parallel `ignored-fallible-results` scan could follow this pattern.

### Script surface

- Existing scripts discovered:
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/scripts/check-source-length.sh`
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/scripts/check-nightly-features.sh`
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/scripts/check-doc-taint-consistency.py`
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/scripts/check-beads-server-mode.sh`
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/scripts/check-agent-cli-contract.sh`
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/scripts/rust-verification-gauntlet.sh`
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/scripts/verify-verus.sh`
  - `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/scripts/verify-lean.sh`
- No dedicated ignored-fallible-results scan script was found by glob.

## Candidate production search domain

Production first-party Rust is primarily under:

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_core/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_yaml/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_validate/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_expr/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_compile/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_storage/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_runtime/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_ipc/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_codegen/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_doc/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_ui_model/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_ui_snapshot/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_ui_makepad/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/vb_proof_kernels/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/velvet_ballistics/src`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/xtask/src`

Non-production search domains that need explicit exclusion or justification:

- `**/tests/**`, `**/*_tests.rs`, `benches/**`, `fuzz/**`, `kani/**`, `verification/**`, `reference/**`, `.beads/**`.
- Existing grep found many `let _ =`, `.ok()`, `drop(Result)`, and `unwrap_or` forms in tests, benches, fuzz, and bead evidence. The new gate must avoid noisy failure on non-production fixtures unless the contract deliberately expands scope.

## Concrete observed risk examples

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/velvet_ballistics/src/main.rs`
  - Grep found `Ok(()) | Err(_) => {}` around lines 4582, 4694, 4697, 4705, and 4708. These look like production silent-discard candidates and need contract/test review. Some may be existing debt tied to blocked `vb-qi37.12.2`; this bead should gate reintroduction and documented patterns without silently accepting them.
  - Grep also found `drop(frame.write_slot(...))` and `drop(journal)` patterns; not every `drop` is a discarded fallible result, so a mechanical gate should type-check or pattern-match narrowly enough to avoid false positives.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/crates/velvet_ballistics/src/storage.rs`
  - Grep found `unwrap_or` on `NonZeroUsize::new(1)`; this is not an ignored fallible `Result` but shows why broad `unwrap_or` scans are too noisy for this bead.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/xtask/src/ui_snapshot.rs` and `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4/xtask/src/loom.rs`
  - Grep found `.ok()` and `filter_map(|e| e.ok())`; these may be intentional lossy conversions in tooling. Contract must decide whether xtask is production first-party for this gate or a tooling exception requiring explicit comments/allowlist.

## Likely implementation seam for later states

No code should be changed in State 2. Later states should evaluate one of these seams:

1. Add a dedicated script such as `scripts/check-ignored-fallible-results.sh` that scans first-party production Rust only, fails on high-confidence silent-discard patterns, and maintains a small explicit allowlist with reason/type/scope.
2. Wire that script into `.moon/tasks/all.yml` as a CI task and into `verify-standard` or an existing canonical Moon aggregate so the gate is reproducible.
3. Strengthen `lint-src` command-line denies to match the master lint contract, especially `-D clippy::let_underscore_must_use`, so crates/targets that do not inherit workspace lints cannot bypass the policy.
4. Optionally add an xtask `Gate::IgnoredFallibleResults` only if command-center gates are still a supported acceptance surface; otherwise avoid expanding stale xtask gate commands.

## Required downstream evidence

- Contract must define exact silent-discard classes: ignored `Result`, `let _ =` on fallible/must-use, `.ok()`/`.err()` lossy conversion when value is unused, wildcard `Err(_) => {}` swallow, `Ok(()) | Err(_) => {}` swallow, and `drop(<fallible>)`.
- Test plan must include negative fixtures proving the gate fails on production-like silent discard and passes on documented non-production exceptions.
- Formal proof likely does not require TLA+/Verus/Kani for a text/static gate unless the gate parser is nontrivial; a lightweight executable proof/test lane is likely sufficient.
- State 11 should run at least `moon run :verify-standard` plus the new/changed gate directly, with regression classification against baseline.

## Risks and blockers

- `vb-qi37.12.2` is currently blocked. Existing production candidates may be known blocked debt. This bead must not claim production is clean unless the gate either fails correctly on current debt or the contract records explicit existing-debt handling.
- Broad regex scanning will create false positives in tests, benches, fuzz, and intentional option conversions. Production-only path filtering and allowlist discipline are high-risk.
- Workspace lints already include `unused_must_use` and `let_underscore_must_use`; the remaining gap is reproducibility across all first-party production surfaces and documented silent-discard patterns that clippy does not catch.
