# Trusted Base Plan: vb-7akm0

This plan enumerates what is trusted (without verification by this bead) versus what
is verified (by the 6 obligations). It is the explicit boundary between **assumed-correct
infrastructure** and **bead-enforced evidence**.

## Trusted (Assumed-Correct Infrastructure)

| ID | Item | Justification |
|----|------|---------------|
| TBP-001 | **`cargo` workspace compilation model** (Cargo.toml:1-65, including `[workspace.lints.rust] unreachable_pub = "deny"` at line 57) | Cargo is the canonical Rust build tool; the workspace lint policy is the lint anchor for this entire bead. The bead does not modify it (delivery-scope.jsonl row 26). |
| TBP-002 | **`vb_validate` crate root lint policy** (`crates/vb_validate/src/lib.rs:3` `#![deny(unreachable_pub)]`) | The crate-level deny makes `#[allow(unreachable_pub)]` overrides meaningful as sub-crate licenses. The bead does not modify it (delivery-scope.jsonl row 27). |
| TBP-003 | **`moon` build orchestrator** (`.moon/tasks/all.yml:46-62`) | `moon run :lint-src` is the canonical lint gate; `cargo clippy --workspace --lib --bins --examples --all-features` is the underlying command. The bead does not modify the moon task (delivery-scope.jsonl row 43). |
| TBP-004 | **`#[cfg(test)] mod` resolution rules** (Rust 2021+ visibility rule) | Items in non-pub modules with default (private) visibility are reachable from any sibling crate-root module via direct path. Verified empirically by `cargo test -p vb_validate --lib` (PO-TEST-001). The Rust reference is authoritative; this bead does not modify Rust semantics. |
| TBP-005 | **`unreachable_pub` lint scope** (Rust compiler built-in) | The lint targets `pub` items without narrowing; `pub(crate)`, `pub(super)`, and `pub(in path)` items are not subject to the lint. Verified by `pub(crate)` narrowing in categories D and `pub(super)` lint-skip in category E.2. The Rust compiler is authoritative. |
| TBP-006 | **Verus production-binding gate** (`scripts/check-verus-production-binding.sh`) | Pre-existing God-Rule-2 gate; the bead does not modify it. Covers H7 production-binding drift risk. |
| TBP-007 | **Verus production_inner drift gate** (`scripts/check-production-inner-drift.sh`) | Pre-existing mirror-drift gate; the bead does not modify it. Pairs with TBP-006 to defend H7. |
| TBP-008 | **Existing Verus specs** (`verification/verus/extern_*.rs`, `verification/verus/production_inner/*.rs`) | God-Rule-2 binding is preserved because the bead does NOT modify these files. `commands_incident::IncidentReport` is local; Verus bindings consume `production::Kind::IncidentReport` enum variant (delivery-scope.jsonl row 32). |
| TBP-009 | **Existing kani harnesses** (`kani/gate_07_stack.rs`, `crates/vb_validate/src/verification/kani_*.rs`) | Pre-existing; consume canonical `vb_validate::gates::validate_gate_XX_*` (delivery-scope.jsonl row 31), NOT the duplicates in `gate_07_stack.rs`…`gate_13_cycles.rs`. Narrowing the duplicates does not affect kani. |
| TBP-010 | **Existing integration test anchors** (`crates/vb_cli/tests/lifecycle_integration.rs`, `crates/workspace_tests/tests/derived_status_replay_timeline_tests.rs:29`) | Pre-existing active tests; the bead's lifecycle.rs allow-removal is safe because these tests consume `vb_cli::lifecycle::test_helpers::create_run_header` (delivery-scope.jsonl rows 35-36). |
| TBP-011 | **Holzman Rust engineering rules** (AGENTS.md: No unsafe, no unwrap, no expect, no panic, no todo, no unimplemented, no dbg) | Pre-existing constraints enforced repo-wide. The bead does not introduce or remove any of these patterns. |
| TBP-012 | **No `pub mod` visibility changes** (`crates/vb_cli/src/lib.rs:6-9` for `commands_diff`, `commands_incident`, `lifecycle`) | The bead does NOT narrow `pub mod` to `pub(crate) mod` (delivery-scope.jsonl row 28 review = "review", not "none"). Only inner items are narrowed. |

## Verified (Bead-Enforced Evidence)

| ID | Item | Verified By |
|----|------|-------------|
| VBP-001 | **Zero `#[allow(unreachable_pub)]` surviving after changes** | PO-LINT-001 (`moon run :lint-src`) |
| VBP-002 | **All 25 narrowed items compile** | PO-COMPILE-001 (`cargo check --workspace --all-features`) |
| VBP-003 | **All tests pass post-change with same count as pre-change baseline** | PO-TEST-001 (`cargo test --workspace`) |
| VBP-004 | **Externally-reachable items remain pub; non-externally-reachable items narrowed correctly** | PO-EXTERN-001 (grep + Verus binding + drift) |
| VBP-005 | **Orphan-test decision recorded before category G changes** | PO-DECISION-001 (`decision-ack.md` artifact existence + content hash) |
| VBP-006 | **Verus production_inner mirror independent of local IncidentReport** | PO-DECISION-GREP-001 (`grep IncidentReport verification/verus/production_inner/` empty) |

## Trust Boundary Summary

The bead inherits 12 trusted infrastructure items (TBP-001..TBP-012) that are
**not verified by this bead** but are assumed correct. The bead enforces 6 evidence
items (VBP-001..VBP-006) that **must be re-verified on every commit** because they
are the only structural guards against the bead's specific failure modes
(missed allow-removal, broken sibling-module visibility, broken external API,
broken Verus binding, broken decision-ack).

If any TBP item is changed by another bead, this bead's plan must be revisited.
In particular, TBP-001 (workspace lint policy) and TBP-003 (moon lint-src task)
are the lint anchors; if either is relaxed, the entire bead becomes vacuous.