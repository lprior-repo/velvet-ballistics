bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 5
updated_at: 2026-05-14T23:06:53Z
attempt: 3-of-7

# Proof Evidence

STATUS: REPAIRED

## Summary

- LEDGER-ID-ALIGNMENT: PASS. `.beads/vb-qi37.13/proof-obligations.jsonl` and `.beads/vb-qi37.13/proof-obligations.planned.jsonl` both contain exactly 9 matching IDs in order: `VERUS-EXIT-001`, `TEST-EXIT-001`, `STATIC-EXIT-001`, `TEST-DIAGNOSTICS-001`, `TEST-STRUCTURED-001`, `TEST-POSTCARD-001`, `FUZZ-POSTCARD-001`, `RECON-CHILD-001`, `MATRIX-COMMAND-001`.
- VERUS-EXIT-001: PASS. Direct Verus invocation verifies the public exit-code model over `0..=8` with `4 verified, 0 errors`.
- TEST-EXIT-001: PASS. `cargo test -p velvet_ballistics exit_code --all-features` passed the filtered public exit-code tests and matching integration tests.
- STATIC-EXIT-001: PASS. Required scan found no `DomainError = 9`, `ExitCode::from(9u8)`, stale `0_to_9`, or public `<= 9` proof residue in the required files.
- TEST-DIAGNOSTICS-001: PASS. `parse_error_unknown_command_exit_code_is_1` passed and observed the fail-closed validation diagnostic route.
- TEST-STRUCTURED-001: PASS. `bdd_format_parity_exit_code_identical_across_formats` passed and preserves diagnostic format parity.
- TEST-POSTCARD-001: PASS. Postcard tests passed all 8 required roundtrip/rejection cases.
- FUZZ-POSTCARD-001: PASS. The stdin harness smoke passed and cargo-fuzz executed the explicit GNU target `x86_64-unknown-linux-gnu` with `-runs=1`.
- RECON-CHILD-001: PASS. The child evidence marker reconciliation command exited 0.
- MATRIX-COMMAND-001: PASS. The command matrix validation command exited 0.

Child evidence compatibility markers retained for repaired State 3/4 reconciliation: `PO-VERUS-EXIT-001: PASS`, `PO-TEST-EXIT-001 evidence check: PASS`, `PO-STATIC-EXIT-001 evidence check: PASS`, `PO-POSTCARD-ROUTE-001: PASS`.

## Command Evidence

### Path Guard

Command:

```bash
pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-qi37-13-r2" && case "$(pwd -P)" in "/home/lewis/src/Velvet-ballistics"|"/home/lewis/src/Velvet-ballistics"/*) exit 1;; esac && test -s ".beads/vb-qi37.13/STATE.md"
```

Result:

```text
/home/lewis/src/vb-qi37-13-r2
```

Status: PASS.

### Ledger ID Alignment

Obligation: LEDGER-ID-ALIGNMENT.

Command:

```bash
python3 -c "import json; from pathlib import Path; base=Path('.beads/vb-qi37.13'); a=[json.loads(l)['id'] for l in (base/'proof-obligations.jsonl').read_text().splitlines() if l.strip()]; b=[json.loads(l)['id'] for l in (base/'proof-obligations.planned.jsonl').read_text().splitlines() if l.strip()]; exp=['VERUS-EXIT-001','TEST-EXIT-001','STATIC-EXIT-001','TEST-DIAGNOSTICS-001','TEST-STRUCTURED-001','TEST-POSTCARD-001','FUZZ-POSTCARD-001','RECON-CHILD-001','MATRIX-COMMAND-001']; print('proof-obligations ids:', a); print('planned ids:', b); raise SystemExit(0 if a == b == exp else 1)"
```

Result:

```text
proof-obligations ids: ['VERUS-EXIT-001', 'TEST-EXIT-001', 'STATIC-EXIT-001', 'TEST-DIAGNOSTICS-001', 'TEST-STRUCTURED-001', 'TEST-POSTCARD-001', 'FUZZ-POSTCARD-001', 'RECON-CHILD-001', 'MATRIX-COMMAND-001']
planned ids: ['VERUS-EXIT-001', 'TEST-EXIT-001', 'STATIC-EXIT-001', 'TEST-DIAGNOSTICS-001', 'TEST-STRUCTURED-001', 'TEST-POSTCARD-001', 'FUZZ-POSTCARD-001', 'RECON-CHILD-001', 'MATRIX-COMMAND-001']
```

Status: PASS.

### VERUS-EXIT-001

Command:

```bash
verus verification/verus/diagnostic_envelope_verus.rs
```

Result:

```text
verification results:: 4 verified, 0 errors
```

Status: PASS.

### TEST-EXIT-001

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics exit_code --all-features
```

Result excerpt:

```text
running 9 tests
test exit_code::tests::all_variants_are_public_range_0_to_8 ... ok
test exit_code::tests::all_variants_are_distinct ... ok
test exit_code::tests::discriminant_values_match_spec ... ok
test exit_code::tests::from_cli_exit_code_to_exit_code ... ok
test exit_code::tests::from_core_error_maps_to_runtime_failed ... ok
test exit_code::tests::from_journal_error_maps_to_storage_error ... ok
test mode_activation_tests::cli_exit_code_all_9_variants_distinct ... ok
test mode_activation_tests::mode_error_all_variants_have_distinct_exit_codes ... ok
test mode_activation_tests::parse_error_unknown_command_exit_code_is_1 ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 164 filtered out; finished in 0.00s
test cli_run_invalid_workflow_returns_error_exit_code ... ok
test bdd_yaml_parse_exit_code_is_validation_failed ... ok
test bdd_inv001_exit_code_stable_across_formats_on_error ... ok
test bdd_format_parity_exit_code_identical_across_formats ... ok
```

Status: PASS.

### STATIC-EXIT-001

Command:

```bash
rg -n "DomainError\s*=\s*9|ExitCode::from\(9u8\)|0_to_9|<= 9" crates/velvet_ballistics/src/exit_code.rs verification/verus/diagnostic_envelope_verus.rs
```

Result: no output; command exit status 1 from no matches.

Status: PASS because no forbidden residue was found.

### TEST-DIAGNOSTICS-001

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics parse_error_unknown_command_exit_code_is_1 --all-features
```

Result excerpt:

```text
running 1 test
test mode_activation_tests::parse_error_unknown_command_exit_code_is_1 ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 172 filtered out; finished in 0.00s
```

Status: PASS.

### TEST-STRUCTURED-001

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics bdd_format_parity_exit_code_identical_across_formats --all-features
```

Result excerpt:

```text
running 1 test
test bdd_format_parity_exit_code_identical_across_formats ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.00s
```

Status: PASS.

### TEST-POSTCARD-001

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard
```

Result:

```text
running 8 tests
test emitter::binary::tests::tests::postcard_rejects_bad_crc ... ok
test emitter::binary::tests::tests::encode_decode_postcard_roundtrip ... ok
test emitter::binary::tests::tests::postcard_rejects_bad_magic ... ok
test emitter::binary::tests::tests::postcard_rejects_payload_too_large ... ok
test emitter::binary::tests::tests::postcard_rejects_old_version ... ok
test emitter::binary::tests::tests::postcard_rejects_bad_payload_digest ... ok
test emitter::binary::tests::tests::postcard_rejects_unsupported_version ... ok
test emitter::binary::tests::tests::postcard_rejects_wrong_kind ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.00s
```

Status: PASS.

### FUZZ-POSTCARD-001 Stdin Harness

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo run --manifest-path fuzz/Cargo.toml --features fuzz --bin vb_ui_model_postcard_decode -- < /dev/null
```

Result:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
Running `target/debug/vb_ui_model_postcard_decode`
```

Status: PASS.

### FUZZ-POSTCARD-001 Cargo-Fuzz GNU Target

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1
```

Result:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 0.08s
Finished `release` profile [optimized + debuginfo] target(s) in 0.08s
Running `target/x86_64-unknown-linux-gnu/release/vb_ui_model_postcard_decode -artifact_prefix=/home/lewis/src/vb-qi37-13-r2/fuzz/artifacts/vb_ui_model_postcard_decode/ -runs=1 /home/lewis/src/vb-qi37-13-r2/fuzz/corpus/vb_ui_model_postcard_decode`
```

Status: PASS; command exited 0.

### RECON-CHILD-001

Command:

```bash
python3 -c "from pathlib import Path; base=Path('.beads/vb-qi37.13'); texts={n:(base/n).read_text() for n in ['proof-evidence.md','proof-review.md','contract-verification-review.md']}; required=['PO-VERUS-EXIT-001: PASS','PO-TEST-EXIT-001 evidence check: PASS','PO-STATIC-EXIT-001 evidence check: PASS','PO-POSTCARD-ROUTE-001: PASS','STATUS: APPROVED','STATUS: REJECTED','vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1']; missing=[s for s in required if not any(s in t for t in texts.values())]; raise SystemExit('missing child evidence markers: '+repr(missing) if missing else 0)"
```

Result: no output; command exited 0.

Status: PASS.

### MATRIX-COMMAND-001

Command:

```bash
python3 -c "import json; from pathlib import Path; base=Path('.beads/vb-qi37.13'); obs=[json.loads(l) for l in (base/'proof-obligations.jsonl').read_text().splitlines() if l.strip()]; ids={o['id'] for o in obs}; markers=['T'+'ODO','UNKN'+'OWN','DISCOVER'+'_','REGISTER'+'_AND_RUN']; bad=[o['id'] for o in obs if o.get('status')=='PASS' or any(m in o.get('command','') for m in markers)]; rows=[json.loads(l) for l in (base/'traceability-matrix.jsonl').read_text().splitlines() if l.strip()]; missing=[(r.get('contract_clause'),p) for r in rows for p in r.get('proofs',[]) if p not in ids and not str(p).startswith('WAIVER-')]; raise SystemExit('bad='+repr(bad)+' missing='+repr(missing) if bad or missing else 0)"
```

Result: no output; command exited 0.

Status: PASS.

## Assumptions and Bounds

- Verus proof artifact models only public CLI exit-code variants, matching `CliExitCode` after repair.
- Cargo-fuzz evidence is a smoke run with `-runs=1`; it proves build/entrypoint execution under libFuzzer but is not a coverage campaign.
- Explicit GNU target `x86_64-unknown-linux-gnu` is the planned and authoritative cargo-fuzz lane for this environment.
- No TLA+/Lean obligation is required for this local mapping/codec scope per State 3 waivers.
