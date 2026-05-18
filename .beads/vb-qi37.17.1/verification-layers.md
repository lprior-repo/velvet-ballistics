# Verification Layers

## Boundary

- **Verus-owned kernel**: Deferred to follow-up bead. No Verus proofs in scope.
- **TLA+ temporal model**: None (no temporal behavior).
- **Theorem projection**: None (Verus would own Rust-local proofs if introduced).
- **Runtime shell**: `cmd_incident` — FjallJournal I/O, JSON/text output.
- **External systems excluded from formal proof**: FjallJournal, serde_json, std::io.

## Layer Assignment

| Contract Clause | Layers |
|-----------------|--------|
| INV-001 (zero-unwrap) | `static-scan` + `cargo-mutants` |
| INV-002 (no stack traces) | `static-scan` + `manual-qa` |
| INV-003 (JSON validity) | `manual-qa` |
| INV-004 (text structure) | `manual-qa` |
| INV-005 (compile correctness) | `static-scan` (clippy) + `moon ci` |
| INV-006 (dead code removal) | `static-scan` (dead_code lint) |
| PRE-001 through PRE-004 | `unit tests` (T-001 through T-013) |
| POST-001 through POST-004 | `unit tests` (T-001 through T-013) + `integration tests` (T-014 through T-016) |

## Static Scan Scope

- `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings`
- Must pass with 0 warnings.
- `cargo check --workspace` must pass with 0 E0061 errors.

## Mutation Testing Scope

- `cargo-mutants` on `vb_cli` crate only (smallest blast radius).
- Target modules: `commands_incident.rs`, `app_impl.rs` (cmd_incident function).
- Expected: mutation kill rate ≥ 90% on touched code.

## Manual QA Scope

- Run `velvet-ballastics incident <run_id> --db <path>` on a test database with:
  1. A failed run — verify JSON output has `failure_code: "RunFailed"`
  2. A non-existent run — verify structured error output
  3. A successful run — verify exit code indicates "not an incident"
- No stack traces visible in any output.

## Waivers

- **Verus**: Deferred — no proof obligation in this bead's scope.
- **Kani**: Not applicable — no unsafe code, no complex state transitions.
- **Miri**: Not required — no unsafe code, no raw pointer manipulation.
- **Proptest**: Not required — pure functions are tested exhaustively with fixed inputs.

---

**Written by**: rust-contract agent
**Bead**: vb-qi37.17.1
**Date**: 2026-05-17
