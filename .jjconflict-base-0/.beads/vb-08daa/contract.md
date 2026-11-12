# Bead vb-08daa: Missing Hunk Recovery from Historical Own Patch

## Contract

This bead recovers 3 missing hunks from historical own-patch `.beads/vb-08daa/historical-own-patch.patch` that are missing from the current tree. The 4th hunk (`&source.inputs()` → `source.inputs()`) is already present in `crates/vb_compile/src/mod_compile_lowering/part_01.rs` and requires no change.

## Missing Hunks to Recover

### Hunk 1: .cargo/config.toml (ci alias)
**Current:**
```toml
ci = "clippy --workspace --all-targets --all-features -- -D warnings"
```
**Target:**
```toml
ci = "clippy --workspace --lib --bins --examples --all-features -- -D warnings"
```
**Reason:** Test clippy is not strict per AGENTS.md. Use source-only targets for `cargo ci` while keeping `--all-features` for feature-gated code and `-- -D warnings` for warning-as-error.

### Hunk 2: contracts/invariants.yaml (no_unsafe_in_first_party scan)
**Current:**
```yaml
no_unsafe_in_first_party:
    - cargo clippy --workspace --all-targets -- -D unsafe_code
```
**Target:**
```yaml
no_unsafe_in_first_party:
    - cargo clippy --workspace --lib --bins --examples -- -D unsafe_code
```
**Reason:** Source-only targets for unsafe code scan, matching the source-lint policy. The existing independent `unsafe-audit`, `panic-surface`, and `banned-token-gates` moon tasks enforce the no-unsafe policy across all first-party code.

### Hunk 3: xtask/src/evidence/persistence.rs (clippy repair hint)
**Current:**
```rust
"clippy" => "cargo +nightly clippy --workspace --all-targets --all-features",
```
**Target:**
```rust
"clippy" => "cargo +nightly clippy --workspace --lib --bins --examples --all-features",
```
**Reason:** Consistent repair command with ci alias target selection.

## Already-Present Hunk (No Change Required)

### Hunk 4: crates/vb_compile/src/mod_compile_lowering/part_01.rs
```rust
// Current and target are identical:
allocate_input_slots(source.inputs(), &mut builder).map_err(|e| CompileErrors(vec![e]))?;
```
**Evidence:** Line 57 of part_01.rs shows `source.inputs()` with no `&` — this hunk was already applied.

## Invariants (No Weakening)

- `--workspace` preserved in all three changes
- `--all-features` preserved in ci alias and persistence repair command
- `-- -D warnings` preserved in ci alias
- `check-all` alias unchanged (still uses `--all-targets` for type checking)
- No runtime-core behavior change; tooling repair hint string literal changed in `xtask/src/evidence/persistence.rs` only
- `no_unsafe_in_first_party` still denies all unsafe code; only target selection changes

## Existing Verification Gates (No New Tests)

- `lint-src` moon task (source gate) runs `cargo clippy --workspace --lib --bins --examples --all-features` with `-D unsafe_code`
- `cargo ci` alias for manual use targets `--lib --bins --examples` matching the source gate
- `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings` — manual smoke
- Existing independent source gates: `panic-surface`, `ignored-fallible-results`, `unsafe-audit`, `banned-token-gates`, `test-integrity`
- `cargo build --workspace --all-features --profile hardened` — hardened build still uses full targets

## Risk Profile

Low. These are tooling/config changes only — no runtime behavior, no API surface, no source attributes. The changes make clippy checks run on source targets instead of all targets (including tests), which aligns with the existing AGENTS.md policy that test clippy is not strict.