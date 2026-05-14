# Verification Layers

## Boundary

- **Verified kernel**: Pure verification logic in `commands_verify.rs` — `run_verification`, `assemble_verification_report`, `repair_hint_for_error`, `exit_code_for_error`. These are deterministic: same input text/bytes/profile always produces same output report.
- **Runtime shell**: CLI command dispatch in `main.rs` (`cmd_verify`), argument parsing in `args.rs`. These handle I/O, file reading, output formatting, and process exit.
- **External systems excluded from formal proof**:
  - Filesystem (workflow file read) — covered by integration tests and manual QA
  - Fjall journal — verify is read-only; `strict` profile checks for journal evidence but does not write
  - Network — none in verify path
  - IPC socket server — none in verify path

## Layer Assignment

| Contract clause | Primary layer | Secondary layer | Notes |
|---|---|---|---|
| PRE-001 (no auth) | `waiver` | — | Auth is trivially absent; no verification layer applies |
| PRE-002 (required inputs) | `proptest` | — | Property test verifies workflow path + profile combinations |
| PRE-003 (system state) | `miri` | `cargo-careful` | UB/aliasing checks on symbol resolution paths |
| POST-001 (VerificationReport emission) | `kani` | `proptest` | Bounded model check on report field completeness; proptest for edge case coverage |
| POST-002 (fail closed on missing durability/replay) | `kani` | `manual-qa` | Kani bounded proof on profile=fail-closed transitions |
| POST-003 (repair hints cite concrete gates) | `proptest` | `manual-qa` | Each error variant maps to at least one non-empty hint |
| INV-001 (stable exit codes) | `cargo-mutants` | `proptest` | Mutation testing proves exit code is format-independent |
| INV-002 (human/machine parity) | `proptest` | `cargo-mutants` | Generated test suite covers all error variants in both formats |
| INV-003 (no panic propagation) | `miri` | `cargo-careful` | UB/panic-free runtime paths verified by Miri on pure crates |
| INV-004 (output completeness) | `kani` | `cargo-fuzz` | Kani on JSON serialization; fuzz on malformed workflow inputs |
| ERR-001 (YamlParse) | `proptest` | `cargo-fuzz` | Fuzz YAML parser; proptest on error message strings |
| ERR-002 (Compile errors) | `proptest` | `cargo-fuzz` | Fuzz on malformed YAML producing compile errors |
| ERR-003 (IrValidation) | `proptest` | `kani` | Property test on IR validation failure paths |
| ERR-004 (BudgetPolicy) | `kani` | `proptest` | Bounded proof on budget computation failure |
| ERR-005 (StorageError) | `manual-qa` | — | Storage errors require filesystem/journal interaction |
| ERR-006 (ReplayDivergence) | `manual-qa` | — | Replay divergence requires journal interaction |
| ERR-007 (VerificationFailed) | `proptest` | `cargo-mutants` | Exit code 2 is produced by multiple error paths |

## Lean Scope

**Not applicable**: verify is a CLI cold path with file I/O, argument parsing, and output formatting. These are not pure deterministic kernels suitable for Lean proof. The pure verification logic (`run_verification`) delegates to existing validated crates (`vb_yaml`, `vb_compile`, `vb_validate`) whose own internal correctness is handled by their respective beads.

**Waiver rationale**: The verification pipeline touches I/O (file read), string formatting, and CLI output rendering — all of which are outside the scope of Lean. The deterministic kernel (verification gate evaluation) is already validated by the existing test suites in `vb_validate`, `vb_compile`, and the CLI integration tests.

## Cargo-Fuzz / Bolero Scope

- **Fuzz target**: Malformed YAML workflow inputs to `run_verification` at all three profile levels
- **Fuzz oracle**: Verify does not panic, returns a classified error (not `panic` or `unwrap` failure), produces non-empty error message
- **Minimum corpus**: 1000 seeds covering valid YAML, invalid YAML, edge cases (empty file, 1MiB file, deeply nested, duplicate keys, forbidden YAML features)

## Loom / Shuttle / Lockbud Scope

Not applicable: verify is single-threaded and synchronous. There are no concurrent interleavings to check in the CLI verify path.

## Cargo-Mutants Scope

- **Target**: `exit_code_for_error` and `assemble_verification_report`
- **Kill mutant examples**:
  - Change `exit_code_for_error` to return `Success` for any error variant → mutant must be killed
  - Change `repair_hint_for_error` to return empty vec for non-empty error → mutant must be killed
  - Change `assemble_verification_report` to drop `exit_code` field → mutant must be killed
- **Baseline**: `moon run :ci` with `cargo mutants` in smoke scope before and after this bead

## Cargo-LLVM-Cov Scope

- Coverage report must show 100% line coverage on `commands_verify.rs`
- Coverage must include all branches of `VerifyError` match arms
- Coverage must include both `OutputFormat::Text` and `OutputFormat::Json` output paths

## Static Scan Scope

- Scan `commands_verify.rs` for: `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, unchecked indexing
- Scan for: hardcoded exit codes that don't match `CliExitCode` discriminant values
- Supply chain: verify all new dependencies added to `velvet_ballastics/Cargo.toml` pass `cargo-vet` and `cargo-deny`

## Manual QA Scope

- **Happy path**: Run `velvet-ballastics verify tests/fixtures/valid/minimal.yaml` in text, JSON, and JSONL modes; verify all certificate fields are present and non-empty
- **Error path**: Run `velvet-ballastics verify tests/fixtures/invalid/invalid_missing_when.yaml`; verify exit code is 1 or 2 (not 0); verify error appears in both text and JSON
- **Full profile fail-closed**: Create a workflow that fails budget policy; run with `--profile full`; verify exit code 2 and non-empty repair hint
- **Format parity**: For each error variant, compare text output and JSON output; both must list the same failing gates
- **Panic containment**: Pass a path that causes a Rust panic in a downstream crate; verify operator sees clean error message, not a backtrace

## Gauntlet Lanes

| Lane | When to use | Evidence produced |
|---|---|---|
| `gauntlet-fast` | Day-to-day CI during implementation | `cargo test`, `cargo clippy`, `cargo fmt` |
| `gauntlet-standard` | PR merge gate | + `cargo mutants` (smoke), `cargo llvm-cov` (line), static scan |
| `gauntlet-deep` | Pre-release or critical path | + `miri` on pure crates, `cargo fuzz` (smoke 10min), proptest 1000 iters |
| `gauntlet-proof` | Formal proof contract | Not applicable (Lean waived) |
| `gauntlet-all` | Release certification | Full gauntlet: deep + all tools + manual QA sign-off |

## Waivers

| Clause | Reason | Compensating evidence |
|---|---|---|
| Lean (all clauses) | CLI cold path with I/O, string formatting, and output rendering outside pure kernel scope | Existing unit/integration tests in `vb_validate`, `vb_compile`; CLI integration tests; manual QA sign-off |
| Loom/Shuttle/Lockbud | verify is single-threaded and synchronous; no concurrent interleavings | N/A |
| `cargo-fuzz` full 1hr run | Verify fuzz is smoke-scoped; full fuzz is covered by `vb_yaml` and `vb_compile` fuzz targets | Fuzz coverage report from `vb_yaml` and `vb_compile` beads |
