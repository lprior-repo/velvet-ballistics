# Test Plan: vb-qi37.13 — CLI Structured Output Contract Reconciliation

## Summary

- STATUS: PLANNED
- Planning scope: `/home/lewis/src/vb-qi37-13-r2` only.
- Behavior sources: `contract.md`, `traceability-matrix.jsonl`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `proof-evidence.md`, `proof-review.md`, and `contract-verification-review.md`.
- State 6 approval preconditions present: `proof-review.md` has `STATUS: APPROVED`; `contract-verification-review.md` has `STATUS: APPROVED`.
- Behaviors identified: 16.
- Trophy allocation: 8 unit / 9 integration / 3 e2e / 3 static-review lanes. Integration dominates because this bead reconciles public CLI/process, structured envelope, proof-ledger, and fuzz-entrypoint boundaries.
- Proptest invariants: 5.
- Fuzz targets: 1 required State 11 lane plus 1 optional corpus-expansion lane.
- Kani harnesses: 4 planned bounded proof candidates.
- Mutation threshold: >=90% kill rate for touched crates; 100% kill required for public exit-code discriminants and postcard error-class branches.

## 0. Preconditions and Approval Gate

The test writer must not start implementation until these checks are true from `/home/lewis/src/vb-qi37-13-r2`:

| Gate | Required assertion | Evidence source |
|---|---|---|
| Path guard | Current directory is exactly `/home/lewis/src/vb-qi37-13-r2`; do not read/write `/home/lewis/src/Velvet-ballistics` or `/home/lewis/src/vb-qi37-13` | State 5/6 path guards |
| Contract exists | `.beads/vb-qi37.13/contract.md` is non-empty | contract.md |
| Traceability exists | `.beads/vb-qi37.13/traceability-matrix.jsonl` is non-empty and valid JSONL | contract-verification review |
| Proof obligations exist | `.beads/vb-qi37.13/proof-obligations.jsonl` and `.beads/vb-qi37.13/proof-obligations.planned.jsonl` are non-empty and contain the same 9 IDs | proof-review.md |
| Proof review approved | `.beads/vb-qi37.13/proof-review.md` contains `STATUS: APPROVED` | proof-review.md |
| Contract verification approved | `.beads/vb-qi37.13/contract-verification-review.md` contains `STATUS: APPROVED` | contract-verification-review.md |
| Existing evidence available | `.beads/vb-qi37.13/proof-evidence.md` records State 5 evidence for exit range, no code 9, diagnostics, structured output, postcard, fuzz, reconciliation, and command matrix | proof-evidence.md |

If any gate is false, return `STATUS: BLOCKED` and do not write tests.

## 1. Behavior Inventory

1. CLI process exits with only public codes `0..=8` when any public command path completes or fails.
2. CLI never exposes `DomainError = 9`, `ExitCode::from(9u8)`, stale `0_to_9`, or public `<= 9` range logic when converting errors.
3. CLI maps validation/parse/unsupported command and mode failures to `ValidationFailed = 1` with typed diagnostics.
4. CLI maps workflow/setup verification failures to `VerificationFailed = 2`.
5. CLI maps compile/codegen/serialization failures to `CompileFailed = 3`.
6. CLI maps runtime/evaluation failures to `RuntimeFailed = 4`.
7. CLI maps storage/journal/persistence failures to `StorageError = 5`.
8. CLI maps IPC failures to `IpcError = 6`.
9. CLI maps action policy/UI initialization failures to `ActionPolicyError = 7`.
10. CLI maps replay divergence to `ReplayDivergence = 8`.
11. Structured output envelopes include stable `schema_version >= 1` and `kind` for every supported structured format.
12. Diagnostic envelopes are machine-readable and include stable `code` and `message`, plus `path`, `span`, and `repair` when context exists.
13. Structured stdout and stderr are separated: successful structured payloads are emitted on stdout; diagnostics/errors are emitted on stderr unless the public command explicitly specifies another documented channel.
14. Postcard decoding validates magic, header length, version, CRC, payload digest, payload bound, and expected kind before exposing payload bytes.
15. Parent evidence reconciliation accepts child evidence markers only as reconciliation facts, not as independent proof laundering.
16. Command matrix rows contain executable or explicit waiver-review commands, never placeholder commands, and never contract-time `PASS` statuses.

## 2. Trophy Allocation

| Layer | Planned coverage | Behaviors | Rationale |
|---|---:|---|---|
| Static analysis / review | 3 lanes | 2, 15, 16 | Static scans and JSONL matrix checks are the cheapest way to catch stale code-9 residue, placeholder commands, unresolved proof references, and improper PASS statuses. |
| Unit / calc | 8 groups | 1-12, 14 | Exact enum discriminants, conversion mappings, diagnostic schema construction, and postcard validation branches must be deterministic and exact. |
| Integration | 9 groups | 1-16 | This bead is primarily boundary reconciliation: CLI parser/output modes, stderr/stdout, command matrix, proof ledgers, fuzz registration, and child artifacts. |
| E2E / black-box | 3 flows | 1, 3, 11-13 | Public operator contract must be observed from CLI/process behavior rather than only internal APIs. |

## 3. BDD Scenarios

### Behavior: public exit codes are exactly 0..=8

- Test name: `fn cli_exit_code_values_are_exactly_zero_through_eight()`
- Given: the public `CliExitCode` taxonomy is available.
- When: every public variant is enumerated and converted to `std::process::ExitCode`.
- Then: the observed discriminant set is exactly `{0,1,2,3,4,5,6,7,8}`.
- And: there is no public variant or conversion result with value `9` or greater.
- Assertion strength: assert the full ordered mapping, not `is_ok()`.

### Behavior: public code 9 is absent from source and proofs

- Test/check name: `static_scan_rejects_public_domain_error_code_nine_residue`
- Given: active public exit-code source and Verus mirror.
- When: scanning for `DomainError\s*=\s*9`, `ExitCode::from(9u8)`, `0_to_9`, or public `<= 9` residue.
- Then: the scan has no matches; `rg` exit status `1` from no matches is interpreted as pass, and any match fails the lane.

### Behavior: validation diagnostics fail closed with code 1

- Test name: `fn parse_error_unknown_command_exit_code_is_1()`
- Given: an unsupported command, unsupported output format, or unsupported emit mode.
- When: the CLI parser validates the request.
- Then: it returns a typed validation diagnostic with public exit code `ValidationFailed = 1`.
- And: diagnostic `code` and `message` are stable and machine-readable.

### Behavior: verification failures map to code 2

- Test name: `fn verification_failure_maps_to_exit_code_2()`
- Given: a workflow/setup verification failure representative.
- When: the failure is converted to `CliExitCode` and then process exit code.
- Then: the exact value is `2` and the taxonomy label is `VerificationFailed`.

### Behavior: compile failures map to code 3

- Test name: `fn compile_failure_maps_to_exit_code_3()`
- Given: a compile/codegen/serialization failure representative.
- When: the failure is converted to public CLI exit code.
- Then: the exact value is `3` and the taxonomy label is `CompileFailed`.

### Behavior: runtime failures map to code 4

- Test name: `fn from_core_error_maps_to_runtime_failed()`
- Given: a `vb_core::errors::CoreError` representative.
- When: the error is converted to `CliExitCode`.
- Then: the exact value is `RuntimeFailed = 4`.

### Behavior: storage failures map to code 5

- Test name: `fn from_journal_error_maps_to_storage_error()`
- Given: a `vb_storage::error::JournalError` representative.
- When: the error is converted to `CliExitCode`.
- Then: the exact value is `StorageError = 5`.

### Behavior: IPC failures map to code 6

- Test name: `fn ipc_error_maps_to_exit_code_6()`
- Given: an IPC client/server failure representative.
- When: the error is converted to public CLI exit code.
- Then: the exact value is `6` and the taxonomy label is `IpcError`.

### Behavior: action policy failures map to code 7

- Test name: `fn action_policy_error_maps_to_exit_code_7()`
- Given: an action policy or UI/action initialization failure representative.
- When: the error is converted to public CLI exit code.
- Then: the exact value is `7` and the taxonomy label is `ActionPolicyError`.

### Behavior: replay divergence maps to code 8

- Test name: `fn replay_divergence_maps_to_exit_code_8()`
- Given: a replay divergence representative.
- When: the error is converted to public CLI exit code.
- Then: the exact value is `8` and the taxonomy label is `ReplayDivergence`.

### Behavior: structured output envelopes preserve schema and kind

- Test name: `fn structured_output_includes_schema_version_and_kind_for_all_formats()`
- Given: each supported structured output format and a representative success payload plus a representative diagnostic payload.
- When: the command emits structured output.
- Then: every envelope includes `schema_version` with value `>= 1` and a stable `kind` string.
- And: exact field names and values are asserted for every supported format.

### Behavior: diagnostic envelope includes contextual fields when available

- Test name: `fn diagnostic_envelope_includes_code_message_path_span_and_repair_when_context_exists()`
- Given: a validation failure with path/span/repair context.
- When: the diagnostic envelope is emitted.
- Then: `code`, `message`, `path`, `span`, and `repair` are present with exact expected values.
- Error variant: when context does not exist, `code` and `message` remain required and contextual fields are absent or null according to the documented schema, never malformed.

### Behavior: stdout and stderr are separated by payload kind

- Test name: `fn cli_writes_success_payload_to_stdout_and_diagnostics_to_stderr()`
- Given: one command that succeeds with structured output and one command that fails validation.
- When: each command is invoked as a process.
- Then: success structured payload bytes appear on stdout and stderr is empty or contains only documented non-structured noise if explicitly allowed.
- And: failure diagnostics appear on stderr with no success payload on stdout.
- And: exit codes match the exact taxonomy.

### Behavior: command matrix covers formats and diagnostic envelope

- Test name: `fn command_matrix_covers_format_parity_and_diagnostic_schema()`
- Given: the command/format matrix from traceability rows.
- When: test-writer runs every listed structured-output command route for supported formats.
- Then: all formats preserve the same public exit code for equivalent diagnostics.
- And: the diagnostic envelope schema remains stable across formats.

### Behavior: postcard decoder rejects malformed headers before exposing payload bytes

- Test names:
  - `fn postcard_rejects_bad_magic()`
  - `fn postcard_rejects_bad_crc()`
  - `fn postcard_rejects_bad_payload_digest()`
  - `fn postcard_rejects_unsupported_version()`
  - `fn postcard_rejects_old_version()`
  - `fn postcard_rejects_payload_too_large()`
  - `fn postcard_rejects_wrong_kind()`
- Given: an encoded postcard envelope with exactly one malformed field per scenario.
- When: `cli_postcard::decode_postcard(data)` or the repository postcard decode route is invoked.
- Then: the exact `PostcardError` variant is returned.
- And: no payload slice is exposed for rejected inputs.
- And: payload-too-large is rejected before allocation or payload exposure.

### Behavior: child evidence reconciliation does not launder proof

- Test/check name: `child_evidence_reconciliation_requires_raw_markers_and_approval_statuses()`
- Given: `proof-evidence.md`, `proof-review.md`, and `contract-verification-review.md`.
- When: the reconciliation command checks required child markers and approval/rejection markers.
- Then: all markers are present.
- And: this check is recorded only as reconciliation evidence; separate direct obligations remain required for exit code, diagnostics, postcard, fuzz, and matrix coverage.

### Behavior: proof command matrix remains executable and waiver-aware

- Test/check name: `proof_command_matrix_rejects_placeholders_pass_status_and_unresolved_refs()`
- Given: current proof obligations and traceability JSONL.
- When: the matrix checker parses rows.
- Then: no row has `status == PASS` at planning time.
- And: no command contains placeholder markers.
- And: every proof reference resolves to a primary obligation ID or explicit `WAIVER-*` rationale row.

## 4. Structured Output Command Matrix

The test writer must build executable cases for each supported command/format/emit combination named by the implementation. The minimum matrix is:

| Scenario | Formats | Exit code | Stdout assertions | Stderr assertions | Envelope assertions |
|---|---|---:|---|---|---|
| Successful structured command | all supported structured formats, including postcard when offered | 0 | Contains one success envelope only | Empty or documented non-diagnostic only | `schema_version >= 1`, `kind` stable, payload kind matches command |
| Unknown command | all supported diagnostic formats | 1 | No success payload | Contains diagnostic envelope | `code`, `message`; path/span/repair only when available |
| Unsupported output format | requested invalid format | 1 | No success payload | Validation diagnostic | exact unsupported-format code/message |
| Unsupported emit mode | requested invalid emit mode | 1 | No success payload | Validation diagnostic | exact unsupported-emit code/message |
| Runtime/core error representative | all structured diagnostic formats | 4 | No success payload | Runtime diagnostic | stable code/message and no code 9 |
| Storage/journal error representative | all structured diagnostic formats | 5 | No success payload | Storage diagnostic | stable code/message and no code 9 |
| Format parity | every supported text/JSON/YAML/postcard-like route in scope | same exact code for same failure | format-specific success channel rules | format-specific diagnostic channel rules | equivalent semantic envelope fields |

Assertions must compare exact values. Tests that only assert `status.success()`, `is_ok()`, or `is_err()` are rejected.

## 5. Postcard Bounded Validation and Error Variants

| Input class | Expected result | Required assertion |
|---|---|---|
| Valid envelope | `Ok((header, payload))` or exact repository success type | Schema version, kind, payload length, digest/CRC accepted exactly |
| Bad magic | `Err(PostcardBadMagic)` | Payload is not exposed |
| Bad CRC | `Err(PostcardBadCrc)` | Payload is not accepted |
| Bad payload digest | `Err(PostcardBadPayloadDigest)` | Payload bytes are not trusted |
| Unsupported future version | `Err(PostcardUnsupportedVersion)` | Future version fails closed |
| Obsolete old version | `Err(PostcardOldVersion)` | Old version fails closed |
| Payload too large | `Err(PostcardPayloadTooLarge)` | Bound is checked before allocation/exposure |
| Wrong kind | `Err(PostcardWrongKind)` | Expected kind mismatch is exact |
| Truncated header | exact header-length error if implemented, otherwise closest typed validation error | No panic and no payload exposure |
| Empty input | exact typed postcard/validation error | No panic and no allocation spike |
| Header length boundary | min valid, max valid, max+1 | Exact accept/reject boundary |

## 6. Proptest Invariants

### Proptest: public exit-code conversion

- Invariant: every generated public `CliExitCode` converts to an integer in `0..=8`.
- Strategy: generate all enum variants or finite integers mapped through a checked constructor.
- Anti-invariant: values outside `0..=8`, especially `9`, must not construct a public exit code.

### Proptest: error taxonomy mapping totality

- Invariant: every generated representative public error class maps to exactly one `CliExitCode` in `1..=8`.
- Strategy: generate one representative per taxonomy class plus unknown/domain-specific business failures.
- Anti-invariant: no generated domain-specific error maps to public code `9`.

### Proptest: structured diagnostic schema stability

- Invariant: diagnostics always serialize with stable `schema_version`, `kind`, `code`, and `message` fields.
- Strategy: arbitrary diagnostic code/message/context combinations within documented bounds.
- Anti-invariant: missing code/message or schema_version `< 1` fails.

### Proptest: postcard encode/decode roundtrip within bounds

- Invariant: any valid bounded envelope encoded then decoded preserves schema version, kind, payload length, and payload bytes.
- Strategy: bounded byte vectors up to the protocol maximum, valid envelope kinds, valid versions.
- Anti-invariant: payload length `max + 1` always yields `PostcardPayloadTooLarge`.

### Proptest: stdout/stderr routing classification

- Invariant: success payload classes route to stdout; diagnostic classes route to stderr; no invocation emits both a success payload and a diagnostic envelope for the same result.
- Strategy: generate command result class, output format, emit mode.
- Anti-invariant: diagnostic on stdout or success payload on stderr fails unless explicitly documented.

## 7. Fuzz Targets

### Required later State 11 lane: `vb_ui_model_postcard_decode`

- Command to preserve exactly: `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1`
- Input type: arbitrary bytes.
- Risk: panic, OOM, payload exposure before validation, wrong-kind acceptance, digest/CRC bypass, schema/kind inconsistency.
- Required corpus seeds: empty input, truncated header, bad magic, unsupported version, old version, bad CRC, bad payload digest, payload length exactly max, payload length max+1, wrong kind, valid minimal envelope, valid maximal bounded envelope.
- Acceptance in State 11: the GNU target builds and runs; longer campaigns may be added, but the `x86_64-unknown-linux-gnu` smoke lane remains mandatory.
- Waiver note: any default musl/ASAN failure is not accepted as PASS for this bead; it may only be documented as a tooling waiver candidate if the GNU lane passes.

### Optional corpus-expansion lane: structured diagnostic parser/emitter

- Input type: arbitrary diagnostic envelope bytes/strings for supported text formats.
- Risk: malformed JSON/YAML/postcard diagnostic envelope panic or schema confusion.
- Status: optional unless implementation exposes a dedicated parser boundary.

## 8. Kani Harnesses

### Kani: exit-code range is bounded

- Property: for every public exit-code variant, the discriminant is `<= 8` and `>= 0`.
- Bound: finite enum exhaustive.
- Rationale: complements Verus proof and catches Rust implementation drift.

### Kani: conversion from public error class is total

- Property: every modeled public error class maps to exactly one exit code in `1..=8` and never panics.
- Bound: finite modeled taxonomy classes.
- Rationale: totality is contract-critical and small enough for bounded checking.

### Kani: postcard payload bound rejects max+1

- Property: lengths `0..=MAX` follow normal header validation and length `MAX+1` rejects before payload exposure.
- Bound: symbolic length around boundary values: `0`, `1`, `MAX-1`, `MAX`, `MAX+1`.
- Rationale: payload bound is security-sensitive and must not rely only on examples.

### Kani: diagnostic schema version lower bound

- Property: every constructed structured diagnostic envelope has `schema_version >= 1`.
- Bound: finite diagnostic variants and optional context combinations.
- Rationale: schema stability is an operator contract.

## 9. Mutation Checkpoints

Minimum threshold: `cargo-mutants` kill rate >=90% for touched crates; the following mutations must be killed:

| Mutation | Must be killed by |
|---|---|
| Change `ReplayDivergence = 8` to `9` | `cli_exit_code_values_are_exactly_zero_through_eight`, static no-code-9 scan |
| Reintroduce `DomainError = 9` | public range tests and static scan |
| Change validation error mapping from `1` to another value | `parse_error_unknown_command_exit_code_is_1` |
| Change core error mapping from `4` | `from_core_error_maps_to_runtime_failed` |
| Change journal error mapping from `5` | `from_journal_error_maps_to_storage_error` |
| Accept unsupported output/emit modes | unsupported mode/format BDD scenarios |
| Remove `schema_version` or `kind` from structured envelopes | structured output schema tests |
| Emit diagnostics on stdout instead of stderr | stdout/stderr separation E2E test |
| Skip postcard magic check | `postcard_rejects_bad_magic` and fuzz corpus |
| Skip CRC or digest check | bad CRC/digest tests and fuzz corpus |
| Change `>` to `>=` or `<` around postcard payload bound | payload boundary tests, proptest, Kani |
| Accept wrong postcard kind | `postcard_rejects_wrong_kind` |
| Allow proof-obligation `PASS` status in planning ledger | matrix command checker |
| Allow placeholder command marker | matrix command checker |

## 10. Combinatorial Coverage Matrix

| Group | Input class | Expected output | Layer |
|---|---|---|---|
| Exit code exact set | all public variants | exact set `0..=8` | unit + Verus + Kani |
| Exit code forbidden 9 | source/proof strings | no forbidden matches | static |
| Error mappings | validation, verification, compile, runtime, storage, IPC, action policy, replay | exact codes `1..=8` | unit/integration |
| Domain-specific error | any business/domain rule violation representative | one of `1..=8`, never `9` | unit/proptest |
| Unknown command | unsupported command | exit `1`, validation diagnostic | integration/E2E |
| Unsupported format | invalid output format | exit `1`, validation diagnostic | integration/E2E |
| Unsupported emit mode | invalid emit mode | exit `1`, validation diagnostic | integration/E2E |
| Structured success | valid command + each supported format | stdout success envelope, exit `0` | integration/E2E |
| Structured diagnostic | invalid command + each supported diagnostic format | stderr diagnostic envelope, exit `1` | integration/E2E |
| Format parity | same failure across formats | same semantic code/message/exit code | integration |
| Diagnostic context present | path/span/repair available | fields present with exact values | unit/integration |
| Diagnostic context absent | no path/span/repair available | stable required fields, optional context absent/null per schema | unit/integration |
| Postcard valid | valid bounded envelope | exact decode success | unit/proptest |
| Postcard bad magic | corrupted magic | `PostcardBadMagic` | unit/fuzz |
| Postcard bad CRC | corrupted CRC | `PostcardBadCrc` | unit/fuzz |
| Postcard bad digest | corrupted payload digest | `PostcardBadPayloadDigest` | unit/fuzz |
| Postcard future version | version above supported max | `PostcardUnsupportedVersion` | unit/fuzz |
| Postcard old version | obsolete version | `PostcardOldVersion` | unit/fuzz |
| Postcard too large | payload length `MAX+1` | `PostcardPayloadTooLarge` before exposure | unit/proptest/Kani/fuzz |
| Postcard wrong kind | expected/actual kind mismatch | `PostcardWrongKind` | unit/fuzz |
| Child reconciliation | required marker set | all markers present but not proof-laundered | static/integration |
| Command matrix | JSONL obligations and trace rows | executable commands, no placeholders, no planning PASS, refs resolved | static/integration |

## 11. Waiver and Non-Applicable Lanes

| Lane | Status | Required handling |
|---|---|---|
| TLA+ temporal model | Non-applicable | Keep waiver rationale: local CLI mapping/codec behavior has no temporal lifecycle/protocol change. No TLA test lane required. |
| Lean/Aeneas/Hax theorem kernel | Non-applicable | Keep waiver rationale: Verus plus executable Rust evidence owns finite enum and bounded codec scope. No theorem-kernel test lane required. |
| Default musl/ASAN cargo-fuzz issue | Waiver candidate only | Must not discharge `FUZZ-POSTCARD-001`; GNU target command remains mandatory. |
| Full workspace release evidence refresh | Non-goal | Do not require full release matrix for this bead; scope to named crates/artifacts and State 11 lanes. |
| New operator commands | Non-goal | Do not add tests expecting new commands; only validate existing supported command/output/emit modes. |

## 12. Required Commands for Test Writer Evidence

Run commands from `/home/lewis/src/vb-qi37-13-r2` only:

```bash
verus verification/verus/diagnostic_envelope_verus.rs
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics exit_code --all-features
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics parse_error_unknown_command_exit_code_is_1 --all-features
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics bdd_format_parity_exit_code_identical_across_formats --all-features
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1
```

Also run the exact static and reconciliation checks recorded in `proof-obligations.jsonl` for `STATIC-EXIT-001`, `RECON-CHILD-001`, and `MATRIX-COMMAND-001`. Treat `rg` no-match exit status `1` as pass only for `STATIC-EXIT-001`; any matching output is failure.

## 13. Exit Criteria

- Every contract clause from `PRE-001..003`, `POST-001..006`, `INV-001..006`, and `ERR-001..016` has at least one explicit BDD or matrix row above.
- Every public exit code is asserted exactly, including success `0` and errors `1..=8`.
- No public `DomainError = 9` or other code outside `0..=8` is accepted.
- Every structured output format in implementation scope has schema/kind and stdout/stderr assertions.
- Every postcard error variant from `ERR-010..016` has exact error-variant tests plus fuzz seeds.
- Child evidence reconciliation is checked without laundering child summaries as direct proof.
- Command matrix and diagnostic envelope checks reject placeholders, unresolved refs, planning-time PASS statuses, and schema drift.
- State 11 keeps the GNU cargo-fuzz postcard lane as required evidence.
- Waiver/non-applicable lanes remain explicit and are not used as substitute PASS evidence.
- No test relies solely on `is_ok()`, `is_err()`, or `status.success()` without exact value assertions.

## Open Questions

- None blocking. Test writer must discover the concrete list of supported output formats and emit modes from the implementation in this checkout and instantiate the command matrix accordingly, without adding new commands or broadening scope.
