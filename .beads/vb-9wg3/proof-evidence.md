# vb-9wg3 Proof Evidence

## Scope

This bead adds complementary exact-width Kani evidence connecting the `specs/tla/BudgetArithmetic.tla` four-limb arithmetic abstraction to Rust integer domains.

Covered claims:

- Every Rust `u64` encodes into four 16-bit limbs and decodes back exactly.
- TLA `WordLE` lexicographic ordering matches Rust `u64 <=` for all symbolic `u64` pairs.
- TLA-style `AddWord` matches Rust `u64::checked_add` for all symbolic `u64` pairs, including exact overflow classification.
- TLA-style `SubWord` matches Rust `u64::checked_sub` for all symbolic `u64` pairs, including exact underflow classification.
- Transcribed TLA U16/U32 field maxima match Rust `u16::MAX` and `u32::MAX`; one-past values exceed the transcribed TLA maxima.
- The existing BudgetArithmetic TLC bounded state model still passes unchanged.

Not covered:

- No PO-030 full validation pipeline composition proof.
- No Gate 8 Verus proof.
- No Gate 8 Miri proof.
- No performance claim.

## Changed Proof Surface

- `crates/vb_core/src/kani_budget_arithmetic_refinement.rs`
- `crates/vb_core/src/lib.rs`

## Raw Evidence

| Lane | Command | Result | Evidence |
| --- | --- | --- | --- |
| Toolchain | `cargo --version`; `rustc --version --verbose`; `rustup show active-toolchain`; `cargo kani --version` | PASS | `.beads/vb-9wg3/evidence/toolchain.out` |
| TLA source hash | `sha256sum specs/tla/BudgetArithmetic.tla` | PASS | `.beads/vb-9wg3/evidence/budget-arithmetic-tla.sha256` |
| TLA transcription map | Manual line mapping from `BudgetArithmetic.tla` to Kani projection | PASS | `.beads/vb-9wg3/tla-transcription-map.md` |
| Kani inventory | `cargo kani --manifest-path crates/vb_core/Cargo.toml list --format json` | PASS, JSON artifact includes 5 new harnesses | `.beads/vb-9wg3/evidence/kani-vb-core-harnesses.json`, `.beads/vb-9wg3/evidence/kani-list-command.out` |
| Kani refinement | `cargo kani --manifest-path crates/vb_core/Cargo.toml --harness tla_word_round_trips_all_rust_u64_values --harness tla_word_order_matches_rust_u64_order --harness tla_add_word_matches_rust_checked_add_for_all_u64 --harness tla_sub_word_matches_rust_checked_sub_for_all_u64 --harness tla_budget_field_widths_match_rust_domains --output-format regular` | PASS, 5/5 harnesses successful | `.beads/vb-9wg3/evidence/kani-budget-arithmetic-refinement.out` |
| TLC | `tlc -config specs/tla/BudgetArithmetic.cfg specs/tla/BudgetArithmetic.tla` | PASS, 166 states generated, 84 distinct states, depth 2 | `.beads/vb-9wg3/evidence/tlc-budget-arithmetic.out` |
| Rust build | `cargo check -p vb_core` | PASS | `.beads/vb-9wg3/evidence/cargo-check-vb-core.out` |
| Workspace CI | `moon ci` | FAIL_OUT_OF_SCOPE_DIRTY_WORKTREE: failures are outside the scoped vb-9wg3 proof files and the checkout contains unrelated dirty files | `.beads/vb-9wg3/evidence/moon-ci.out` |

## Kani Harness Map

| Harness | Claim | Domain |
| --- | --- | --- |
| `tla_word_round_trips_all_rust_u64_values` | Four-limb encoding is exact | all symbolic `u64` |
| `tla_word_order_matches_rust_u64_order` | TLA lexicographic word order matches Rust order | all symbolic `u64` pairs |
| `tla_add_word_matches_rust_checked_add_for_all_u64` | TLA `AddWord` matches Rust checked addition and overflow | all symbolic `u64` pairs |
| `tla_sub_word_matches_rust_checked_sub_for_all_u64` | TLA `SubWord` matches Rust checked subtraction and underflow | all symbolic `u64` pairs |
| `tla_budget_field_widths_match_rust_domains` | Transcribed TLA field maxima match Rust `u16` and `u32` domains | all symbolic `u16` and `u32` values plus one-past boundaries |

## Proof Hygiene

- No `kani::assume` in the new harness.
- No stubs, function contracts, or experimental `-Z` flags.
- No disabled Kani safety checks.
- No unsafe code.
- Kani reports some unreachable internal `Result`/`Option` branches for limb helper overflow/underflow paths; these are expected because the symbolic inputs are decoded into 16-bit limbs before helper arithmetic. The harness-level refinement assertions all pass, including `WordTypeOK` preservation on add/sub success paths.

## Residual Risk

The Kani harness is a reviewed Rust projection of the TLA equations, documented in `.beads/vb-9wg3/tla-transcription-map.md`, rather than a parser-backed proof over the `.tla` file. If `BudgetArithmetic.tla` changes, this refinement harness and transcription map must be updated and rerun with TLC.

Workspace `moon ci` remains red in a dirty checkout for failures outside the touched proof files. The scoped bead evidence passed.
