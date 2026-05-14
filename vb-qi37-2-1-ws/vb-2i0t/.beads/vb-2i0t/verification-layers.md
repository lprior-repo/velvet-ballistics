# Verification Layers

## Boundary

- **Verified kernel**: The xtask command wrappers in `xtask/src/main.rs` plus evidence-bundle types in `xtask/src/evidence.rs` (new module). The pure kernel is the evidence serialization, gate orchestration, and why-failed diagnostic logic.
- **Runtime shell**: The `cargo xtask <subcommand>` CLI entrypoint which delegates to the evidence module.
- **External systems excluded from formal proof**: The underlying tools (fmt, clippy, nextest, miri, cargo, moon, just) are treated as oracles. Only the wrapper's handling of their output is verified.

---

## Layer Assignment

### Precondition Verification

| Contract Clause | Verification Layer | Evidence |
|----------------|-------------------|----------|
| PRE-001 (gate inventory) | static-scan + manual-qa | `xtask/src/main.rs` imports confirm all gates are covered; manual review of MASTER.md Section 77 gate list |
| PRE-002 (gate list extraction) | unit-test | `test_gate_list_exhaustive` enumerates all 28 Section 77 gates and asserts each has an xtask variant or explicit blocker |
| PRE-003 (xtask builds) | moon ci | `moon run :check` already gates this; no new proof needed |
| PRE-004 (toolchain available) | static-scan | `which cargo`, `which rustup` checks in CI entrypoint script |

### Postcondition Verification

| Contract Clause | Verification Layer | Evidence |
|----------------|-------------------|----------|
| POST-001 (ai-fast gates wrapped) | unit-test + static-scan | `test_ai_fast_gates_all_wrapped` enumerates 6 gates; `test_no_unwrapped_gate_in_main` greps for missing match arms |
| POST-002 (ai-deep gates wrapped) | unit-test + static-scan | `test_ai_deep_gates_all_wrapped` enumerates 4 gates |
| POST-003 (ai-release gates wrapped) | unit-test + static-scan | `test_ai_release_gates_all_wrapped` enumerates 11 gates |
| POST-004 (evidence bundle fields) | proptest | `proptest!` generates arbitrary GateEvidence and asserts all required fields serialize/deserialize round-trip |
| POST-005 (why-failed structure) | unit-test | `test_why_failed_fields_present` constructs failed evidence and asserts hint + repair_command are populated |
| POST-006 (evidence path scoping) | unit-test + integration-test | `test_evidence_goes_to_bead_dir` and integration test with `XBEAK_ID=test` |
| POST-007 (aggregate profile command) | integration-test | `test_ai_fast_profile_aggregates_evidence` runs `cargo xtask ai-fast --bead vb-test` and inspects output YAML |
| POST-008 (exit code semantics) | integration-test | `test_exit_code_0_on_pass` and `test_exit_code_1_on_fail` exercise both paths |
| POST-009 (bead flag) | unit-test + integration-test | `test_bead_flag_propagates` |

### Invariant Verification

| Contract Clause | Verification Layer | Evidence |
|----------------|-------------------|----------|
| INV-001 (fail closed) | unit-test + cargo-mutants | `test_missing_evidence_is_failure`; cargo-mutants on evidence path construction to catch early-return bugs |
| INV-002 (bounded fast) | static-scan + kani | kani on timeout arithmetic; static scan for loop bounds in gate runner |
| INV-003 (deterministic) | proptest | `proptest!` with fixed seed runs same gate twice and compares evidence digests |
| INV-004 (no panic) | kani + static-scan | kani on all `Result` paths in evidence module; static scan for `panic!`, `unwrap!`, `expect!` in xtask |
| INV-005 (structured output only) | integration-test | `test_no_raw_tool_output_on_stdout` — stdout must be valid YAML, raw tool output goes to log files only |
| INV-006 (agent-executable) | manual-qa | No interactive prompts, no TTY required, all flags have defaults; verified by running profile without user input |

### Error Taxonomy Verification

| Error Variant | Verification Layer | Evidence |
|--------------|-------------------|----------|
| GateTimeout | kani + unit-test | kani on duration arithmetic; unit test with artificial timeout |
| GateFailed | unit-test + integration-test | `test_gate_failed_propagates` |
| MissingEvidence | unit-test + cargo-mutants | `test_missing_evidence_fails`; cargo-mutants on path existence check |
| EvidenceWriteFailed | unit-test | mock filesystem failure |
| SubcommandNotFound | unit-test | `test_unknown_subcommand` |
| BeadDirectoryCreationFailed | unit-test | mock dir creation failure |
| YamlSerializationFailed | proptest | proptest with arbitrary bytes in fields |
| UpstreamMoonFailed | integration-test | mock moon failure |
| UpstreamJustFailed | integration-test | mock just failure |

---

## Lean Scope

**Not applicable.** This bead is about CLI orchestration and YAML serialization, not pure mathematical kernels. Lean is not the right tool for:

- Subprocess exit-code handling
- YAML serialization edge cases
- Filesystem path manipulation
- CLI argument parsing

Lean would be appropriate only if we extracted a pure evidence-bundle schema validator as a standalone mathematical artifact — but that is not the primary deliverable.

**Waiver reason**: The atomized gates are a system-integration concern (CLI + subprocess + filesystem + YAML), not a puredeterministic kernel amenable to Lean proof. The evidence schema itself could theoretically be proven correct as a data structure, but the cost/benefit does not justify a separate Lean module for this bead.

---

## Fuzzing Scope

- **Evidence bundle YAML parsing** — bolero harness generates arbitrary YAML and asserts deserialization either succeeds with valid `GateEvidence` or returns a typed error; must never panic
- **Gate name strings** — fuzz arbitrary gate names through the CLI argument parser
- **Evidence path traversal** — fuzz path components to ensure no path traversal escapes the evidence directory

**Tools**: `cargo fuzz` (bolero) for YAML parsing; `cargo-llvm-cov` for coverage of the evidence module.

---

## Concurrency Scope

- Subprocess spawning is sequential (gates run one at a time in ai-fast/ai-deep/ai-release)
- No shared mutable state between gate executions
- No async I/O in the xtask wrapper itself

**Waiver**: Loom/Shuttle not required. The sequential gate-runner model has no concurrency surface.

---

## Waivers

| Clause | Reason | Compensating Evidence |
|--------|--------|----------------------|
| Lean (general) | CLI/subprocess/serialization domain; not a pure kernel | unit + integration tests + kani + proptest + cargo-mutants |
| Loom/Shuttle | Sequential gate execution; no concurrent shared state | Unit tests + integration tests verify sequential correctness |
| Miri | No unsafe code in xtask wrappers; `forbid(unsafe_code)` in xtask | static-scan + kani |
