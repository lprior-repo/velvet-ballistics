# Lean Contract Projection — vb-qi37.12.1

## Boundary

### Lean-Owned Kernel
None. This is a verification-only audit bead. No new pure deterministic kernels, algorithms, state machines, protocol lattices, arithmetic bounds, parsers, codecs, or critical data structures are introduced.

### Rust/Runtime Shell
The entire audit scope (production code in `vb_storage`, `vb_runtime`, `vb_core`, `vb_expr`, `vb_validate`, `vb_compile`, `vb_ipc`) is verified by inspection and existing lint/verification gates. The shell behavior around any pure contracts was already proven in prior beads.

### External Systems Excluded from Lean Proof
- Fjall persistence (verified by integration tests + Miri)
- Binary IPC protocol (verified by fuzz + integration tests)
- External process ingress (verified by manual QA)

## Lean-Owned Clauses

None. This bead is a **negative audit** — it documents that no silent discard patterns exist in production code. No new pure deterministic behavior requires Lean projection.

## Theorem Obligations

None. The pure deterministic behavior verified by this audit was already subject to Lean obligations in the beads that introduced the original code. This bead merely confirms the invariant holds.

## Waivers

### WAIVER-LEAN-001

- **Clause IDs**: AUDIT-001, AUDIT-002, AUDIT-003, AUDIT-004, AUDIT-005, INV-SILENCE-001, INV-SILENCE-002
- **Owner**: vb-qi37.12.1 contract synthesizer
- **Reason**: Verification-only audit bead. No new pure deterministic critical behavior introduced. All audited code was already verified by prior Lean obligations or will be verified by existing gates (clippy, cargo-miri, cargo-fuzz, cargo-mutants).
- **Expiry**: Never — waiver is permanent for this bead's scope
- **Compensating Evidence**:
  - `clippy::unwrap_used = "deny"` in clippy.toml / `deny(unwrap_used)` in source
  - `clippy::expect_used = "deny"` in clippy.toml / `deny(expect_used)` in source
  - `clippy::panic = "deny"` in clippy.toml / `deny(panic_in_result_fn)` in source
  - All `.unwrap()`, `.expect()`, `panic!` occurrences are exclusively in `#[cfg(test)]` modules verified by grep audit
  - `cargo-miri` runs on `vb_core`, `vb_expr` crates for UB detection
  - `cargo-fuzz` fuzzes parser, codec, and IPC boundaries
  - `cargo-mutants` mutation tests verify test quality

### WAIVER-LEAN-002

- **Clause ID**: AUDIT-004 (ignored Results)
- **Owner**: vb-qi37.12.1 contract synthesizer
- **Reason**: Ignored Result detection is enforced by clippy lint gates (`result_expect`, `result_unwrap_or`, `unnecessary_to_owned`, `unused_result`) rather than Lean proofs. These are denial-of-service style lints that catch pattern matches at compile time.
- **Expiry**: Never
- **Compensating Evidence**:
  - `cargo clippy -- -D clippy::result_expect -D clippy::unwrap_used` in CI gate
  - `cargo clippy -- -D clippy::unused_result` in CI gate

## Verification Contract

Since this bead introduces no new code requiring proof:

1. The audit confirms existing Lean obligations remain valid
2. No new theorems required
3. No refinement relations to establish
4. No shell exclusions to document beyond existing contracts

---

**Lean Contract Status**: WAIVED — No Lean obligations arise from this verification-only audit bead. All verified behavior is already covered by existing contracts or lint gates.