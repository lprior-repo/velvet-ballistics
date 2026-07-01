# Proof Coverage Matrix — vb-tsjnz

STATUS: PLANNED (proof-planner State 4). No proof closure or PASS is claimed.

## Coverage by requirement

`OK` denotes covered by a planned obligation; `Δ` denotes a covered-by-other
process lane (diff-audit supporting); `N/A` denotes the verifier lane is
not relevant for the requirement; blank denotes uncovered.

| Requirement | Contract clause | cargo-check | cargo-clippy | cargo-test | cargo-metadata | diff-audit | N/A lanes |
|---|---|---|---|---|---|---|---|
| REQ-VBTSJNZ-001 | `version.workspace = true` on line 3 | OK PO-001 |   |   | OK PO-004 | OK PO-004 | kani, verus, flux-rs, loom, miri, proptest, cargo-fuzz |
| REQ-VBTSJNZ-002 | `[lints]\nworkspace = true` is the final block | OK PO-001 | OK PO-002 |   | OK PO-004 | OK PO-004 | kani, verus, flux-rs, loom, miri, proptest, cargo-fuzz |
| REQ-VBTSJNZ-003 | held invariants `edition.workspace`, `license.workspace`, `publish = false` |   |   |   |   | OK PO-004 | all tooling lanes |
| REQ-VBTSJNZ-004 | sibling-pattern parity |   |   |   |   | OK PO-004 | all tooling lanes |
| REQ-VBTSJNZ-005 | `cargo check ... --all-targets` exits 0 | OK PO-001 | OK PO-002 |   |   |   | kani, verus, flux-rs, loom, miri, proptest, cargo-fuzz |
| REQ-VBTSJNZ-006 | `cargo clippy ... -- -D warnings` exits 0 |   | OK PO-002 |   |   |   | kani, verus, flux-rs, loom, miri, proptest, cargo-fuzz |
| REQ-VBTSJNZ-007 | workspace_tests assertions exit 0 |   |   | OK PO-003 |   |   | kani, verus, flux-rs, loom, miri, proptest, cargo-fuzz |
| REQ-VBTSJNZ-008 | diff bounded to two hunks |   |   |   |   | OK PO-004 | all tooling lanes |
| REQ-VBTSJNZ-009 | `.config/source-length-exceptions.txt` line 323 unchanged |   |   |   |   | OK PO-004 | all tooling lanes |
| REQ-VBTSJNZ-010 | (recovery under failure) | OK PO-001 | OK PO-002 |   |   |   | (process; not a verifier lane) |
| REQ-VBTSJNZ-011 | cargo metadata reports equal versions |   |   |   | OK PO-004 |   | kani, verus, flux-rs, loom, miri, proptest, cargo-fuzz |
| REQ-VBTSJNZ-012 | black-hat reviewer audit (post-landing) |   |   |   |   | OK PO-004 supporting | (post-landing reviewer) |

## Coverage by obligation

| Obligation ID | Verifier | Requirements covered |
|---|---|---|
| PO-VBTSJNZ-001 | cargo-check | REQ-VBTSJNZ-001, -002, -005, -010 |
| PO-VBTSJNZ-002 | cargo-clippy | REQ-VBTSJNZ-002, -005, -006, -010 |
| PO-VBTSJNZ-003 | cargo-test | REQ-VBTSJNZ-007 |
| PO-VBTSJNZ-004 | cargo-metadata + diff-audit | REQ-VBTSJNZ-001, -002, -003, -004, -008, -009, -011, -012 |

## Lanes per requirement (canonical)

| Requirement | Required lanes / obligations | Notes |
|---|---|---|
| REQ-VBTSJNZ-001 | cargo-metadata (PO-004); cargo-check (PO-001 supporting); diff-audit (PO-004 supporting) | metadata-only |
| REQ-VBTSJNZ-002 | cargo-check (PO-001); cargo-clippy (PO-002); cargo-metadata (PO-004 supporting); diff-audit (PO-004 supporting) | lint policy change is the primary risk |
| REQ-VBTSJNZ-003 | diff-audit (PO-004 supporting) | held invariants |
| REQ-VBTSJNZ-004 | diff-audit (PO-004 supporting) | sibling parity |
| REQ-VBTSJNZ-005 | cargo-check (PO-001); cargo-clippy (PO-002) | build acceptance |
| REQ-VBTSJNZ-006 | cargo-clippy (PO-002) | zero-warning |
| REQ-VBTSJNZ-007 | cargo-test (PO-003) | existing test suite |
| REQ-VBTSJNZ-008 | diff-audit (PO-004 supporting) | bounded hunks |
| REQ-VBTSJNZ-009 | diff-audit (PO-004 supporting) | exception preserved |
| REQ-VBTSJNZ-010 | cargo-check (PO-001) + cargo-clippy (PO-002) | recovery is a process rule, not a verifier; the build gates confirm whether recovery is needed |
| REQ-VBTSJNZ-011 | cargo-metadata (PO-004) | version equality |
| REQ-VBTSJNZ-012 | diff-audit (PO-004 supporting) + reviewer judgment | post-landing review |

## Coverage gaps

None. Every requirement is mapped to at least one obligation. The
risk surface is small (one Cargo.toml) and the entire surface is
covered by the four obligations plus diff-audit supporting evidence.

## N/A lanes by requirement

For every requirement the following lanes are recorded
`not_applicable` per EARS:

- **kani** — Cargo metadata-only patch; no bounded Rust control-flow
  surface in scope. `lib.rs` is out-of-scope (owned by `vb-2lu1`).
  Source: `crates/vb_queue_semantics/src/lib.rs` (out-of-scope).
- **verus** — No production-bound Verus seam exists for a Cargo.toml
  patch; mirror-only models would violate the no-vacuum-Verus mandate.
  Source: AGENTS.md formal verification mandates; `crates/vb_queue_semantics/Cargo.toml`.
- **flux-rs** — No scoped Flux annotations exist; the patch is a TOML
  edit, not a Rust function with refinement bounds.
- **loom** — No concurrency introduced.
- **miri** — No unsafe code.
- **proptest** — No behavioral property to test.
- **cargo-fuzz** — No parser/codec/hostile-input surface.

## Waiver impact

`waiver-candidates.jsonl` contains zero rows. The bead is metadata-only
and the patch must not relax any policy that would create a need for a
waiver. PO-001 / PO-002 failures are not waived; they trigger the
Holzman-Rust recovery rule (`Failed::LintFailure`) and become a
follow-up bead owned by the original `lib.rs` author.

## Behavior-affecting rows

Rows marked `behavior_affecting: true`:

- PO-VBTSJNZ-001 — true: enabling workspace lints changes the build
  acceptance surface for `crates/vb_queue_semantics/src/lib.rs`.
- PO-VBTSJNZ-002 — true: same reason as PO-VBTSJNZ-001.
- PO-VBTSJNZ-003 — false: existing workspace_tests do not exercise any
  new behavior surface; package-name and member-name smoke only.
- PO-VBTSJNZ-004 — false: cargo-metadata + diff-audit both observe
  resolution and patch shape; neither is a behavioral surface.
