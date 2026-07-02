# Proof Strategy — vb-tsjnz

STATUS: PLANNED (proof-planner State 4). No verifier, test, fuzz, CI, or
proof success is claimed here. The proof-plan-reviewer owns disposition;
formal-verifier owns closure.

## Bead

- **ID:** `vb-tsjnz`
- **Title:** Cargo: opt `vb_queue_semantics` into workspace lints and version (P1)
- **Type:** Cargo metadata-only patch (no source, no API, no behavior change)
- **Patch:** exactly two hunks in `crates/vb_queue_semantics/Cargo.toml`
  1. line 3 `version = "0.1.0"` → `version.workspace = true`
  2. trailing two lines appended: `[lints]` / `workspace = true`

## Risk classification

The bead is a pure Cargo manifest patch. The risk surface collapses to:

| Risk class | In scope | Notes |
|---|---|---|
| Cargo-metadata | **YES** | version inheritance + `[lints]` opt-in resolution |
| Lint policy | **YES** | workspace lints become effective for `vb_queue_semantics/src/lib.rs` |
| Out-of-scope bleed | **YES** | touching `src/lib.rs`, removing `vb-2lu1` exception, or touching other workspace lints is forbidden |
| Public API / semver | **NO** | resolved version remains `"0.1.0"`; semver-equivalent |
| Concurrency / unsafe / UB | **NO** | no code change; no new locks, raw pointers, atomics, FFI |
| Parser / hostile input | **NO** | no parser/codec path |
| Network / IO | **NO** | no I/O surface |
| Performance | **NO** | no codepath change |
| Dependency / supply-chain | **NO** | no dep change |
| Refinement / type-state | **NO** | no new types |
| Temporal / state-machine | **NO** | the stub has no behavior |

Risk label summary: `cargo-metadata`, `lint-policy`, `compile-acceptance`,
`process-hazard`, `test-coverage`, `source-length`, `out-of-scope`,
`out-of-scope-bleed`, `public-api` (no-drift), `dependency` (no-change).

## Verifier selection

This bead is behavior-preserving Cargo metadata. Per the skill's lane
contract, the following lanes are required:

| Lane | Required? | Reason |
|---|---|---|
| cargo-metadata | **YES** | REQ-VBTSJNZ-011; PS-VBTSJNZ-007 — confirm version inheritance resolves to `[workspace.package].version = "0.1.0"` |
| cargo-check | **YES** | REQ-VBTSJNZ-005; PS-VBTSJNZ-003 — confirm crate compiles under workspace lints after opt-in |
| cargo-clippy | **YES** | REQ-VBTSJNZ-006; PS-VBTSJNZ-004 — confirm `-D warnings` is clean; covers correctness/suspicious/perf/complexity clippy groups |
| cargo-test | **YES** | REQ-VBTSJNZ-007; PS-VBTSJNZ-005 — run `vb_8ma2_workspace_assertions` and `vb_qi37_25_quality_gates` |
| diff-audit | **YES** | REQ-VBTSJNZ-008 / -009; PS-VBTSJNZ-006 / -008 — `jj diff` confirms exactly one file modified and exception file untouched |

Non-applicable lanes (recorded explicitly per EARS, with concrete evidence
sources, never silently omitted):

| Lane | Required? | Reason (with source ref) |
|---|---|---|
| kani | NO | Cargo metadata-only patch; no bounded Rust control-flow surface in scope. Behavior, if any, would have been already in the stub. Source: `crates/vb_queue_semantics/src/lib.rs` is the only Rust surface and is out-of-scope for vb-tsjnz (vb-2lu1). |
| verus | NO | No production-bound Verus seam exists for a Cargo.toml patch; mirror-only models would violate the no-vacuum-Verus rule. |
| flux-rs | NO | No scoped Flux annotations or extern specs in this bead; nothing to refine. |
| loom | NO | No concurrency introduced; no new shared state; no locks. |
| miri | NO | No unsafe code; no new raw pointers; no new FFI; out-of-scope for Cargo manifest patch. |
| proptest | NO | No behavioral property to test; the stub has no runtime to property-check. |
| cargo-fuzz | NO | No parser/codec/hostile input in the patch; the unaffected `src/lib.rs` is out-of-scope. |

## Requirement coverage

| Requirement | Lanes / obligations |
|---|---|
| REQ-VBTSJNZ-001 (version.workspace) | cargo-metadata (PO-004); diff-audit (covers PS-VBTSJNZ-001) |
| REQ-VBTSJNZ-002 ([lints].workspace) | cargo-check (PO-001); cargo-clippy (PO-002); diff-audit |
| REQ-VBTSJNZ-003 (held invariants) | diff-audit (PO-004 supporting) |
| REQ-VBTSJNZ-004 (sibling parity) | diff-audit (PO-004 supporting) |
| REQ-VBTSJNZ-005 (cargo check green) | cargo-check (PO-001) |
| REQ-VBTSJNZ-006 (cargo clippy zero-warning) | cargo-clippy (PO-002) |
| REQ-VBTSJNZ-007 (workspace_tests pass) | cargo-test (PO-003) |
| REQ-VBTSJNZ-008 (diff bounded) | diff-audit (PO-004 supporting) |
| REQ-VBTSJNZ-009 (vb-2lu1 untouched) | diff-audit (PO-004 supporting) |
| REQ-VBTSJNZ-011 (cargo metadata parity) | cargo-metadata (PO-004) |

## Proof architecture

```
vb-tsjnz Cargo metadata patch
├── PO-001 cargo-check
│   └── cargo check -p vb_queue_semantics --all-targets  →  exit 0
├── PO-002 cargo-clippy
│   └── cargo clippy -p vb_queue_semantics --all-targets -- -D warnings
│       →  exit 0
├── PO-003 cargo-test (workspace_tests)
│   ├── cargo test -p workspace_tests --test vb_8ma2_workspace_assertions
│   └── cargo test -p workspace_tests --test vb_qi37_25_quality_gates
├── PO-004 cargo-metadata + diff-audit
│   ├── cargo metadata --no-deps --format-version 1
│   │     .packages[] | select(.name=="vb_queue_semantics").version == "0.1.0"
│   └── jj diff shows one file changed: crates/vb_queue_semantics/Cargo.toml
│         - line 3 replaced; trailing two lines appended
│         - .config/source-length-exceptions.txt UNCHANGED
└── NOT-APPLICABLE: kani, verus, flux-rs, loom, miri, proptest, cargo-fuzz
```

## Key bounds and limits

- File scope: only `crates/vb_queue_semantics/Cargo.toml` may change.
- Two hunks only: line 3 replacement + tail block append.
- Resolved version MUST equal `"0.1.0"` (current literal value).
- `[lints]\nworkspace = true` MUST be the final block in the file.
- `edition.workspace = true`, `license.workspace = true`, `publish = false`
  MUST remain unchanged.

## Forbidden repairs (cross-cutting — restated for the planning layer)

The proof plan MUST NOT recommend, plan, or carry a waiver that:

- Lowers the priority of any workspace lint.
- Removes any workspace lint.
- Adds `#[allow(...)]` to `crates/vb_queue_semantics/src/lib.rs`.
- Edits `.config/source-length-exceptions.txt` to remove the `vb-2lu1`
  entry.
- Edits `rust-toolchain.toml` to bypass the patch.
- Edits contract artifacts retroactively.

If PO-001 or PO-002 fails, recovery follows Holzman-Rust rule
`Failed::LintFailure`: the patch does NOT land; the source cleanup is
handed to a follow-up bead owned by the original `lib.rs` author. No
behavior-affecting waiver is filed against the workspace policy.

## Waiver posture

`waiver-candidates.jsonl` contains zero rows. The bead is metadata-only
and the patch must not relax any policy that would create a need for a
waiver. Any future lapse into a `Failed::LintFailure` becomes a
follow-up bead, not a waiver.

## Handoff

- `proof-plan-reviewer` (State 4b) dispositions each lane decision in
  `verifier-lane-decisions.jsonl` and writes `verifier-lane-review.jsonl`
  + `proof-plan-review.md`. Those files are out of scope for this
  artifact set.
- `holzman-rust` (State 5) executes the four obligations after the
  Cargo.toml edit lands.
- `black-hat-reviewer` (State 6) verifies both axes (version inheritance
  + lints inheritance) by reading the diff and re-running cargo
  metadata / cargo clippy.
- `landing-skill` (State 7+) gates on green PO-001..PO-004.
