bead_id: vb-qi37.13
phase: 6
reviewer: proof-reviewer
status: APPROVED
finding_count: 0
routing: proceed_to_next_state

# State 6 Proof Review Rerun

STATUS: APPROVED

## Findings

No blocking proof findings.

## Scope Guard

- Review performed only in `/home/lewis/src/vb-qi37-13-r2`.
- Forbidden source checkout `/home/lewis/src/Velvet-ballistics` was not used.
- Broken partial checkout `/home/lewis/src/vb-qi37-13` was not used.
- Artifacts written by this review are limited to `.beads/vb-qi37.13/proof-review.md`, `.beads/vb-qi37.13/proof-findings.jsonl`, and `.beads/vb-qi37.13/proof-repair-guide.md`.

Path/artifact guard rerun from `/home/lewis/src/vb-qi37-13-r2`:

```bash
test -s ".beads/vb-qi37.13/proof-obligations.jsonl" && test -s ".beads/vb-qi37.13/proof-obligations.planned.jsonl" && test -s ".beads/vb-qi37.13/proof-writer-report.md" && test -s ".beads/vb-qi37.13/proof-evidence.md"
```

Observed output: none. Exit status: 0.

## Ledger Review

Reviewed current 9-row proof plan and evidence:

- `VERUS-EXIT-001`
- `TEST-EXIT-001`
- `STATIC-EXIT-001`
- `TEST-DIAGNOSTICS-001`
- `TEST-STRUCTURED-001`
- `TEST-POSTCARD-001`
- `FUZZ-POSTCARD-001`
- `RECON-CHILD-001`
- `MATRIX-COMMAND-001`

Reviewer parity check over both `.beads/vb-qi37.13/proof-obligations.jsonl` and `.beads/vb-qi37.13/proof-obligations.planned.jsonl`:

```bash
python3 -c "import json; from pathlib import Path; base=Path('.beads/vb-qi37.13'); expected=['VERUS-EXIT-001','TEST-EXIT-001','STATIC-EXIT-001','TEST-DIAGNOSTICS-001','TEST-STRUCTURED-001','TEST-POSTCARD-001','FUZZ-POSTCARD-001','RECON-CHILD-001','MATRIX-COMMAND-001']; markers=['T'+'ODO','UNKN'+'OWN','DISCOVER'+'_','REGISTER'+'_AND_RUN']; files=['proof-obligations.jsonl','proof-obligations.planned.jsonl']; report=[]; bad=[]; rows_by_file={};
for name in files:
 rows=[json.loads(l) for l in (base/name).read_text().splitlines() if l.strip()]; rows_by_file[name]=rows; ids=[r['id'] for r in rows]; report.append(f'{name}: '+','.join(ids));
 if ids != expected: bad.append((name,'ids',ids));
 for r in rows:
  if r.get('status') == 'PASS' or any(m in r.get('command','') for m in markers): bad.append((name,r.get('id'),r.get('command'),r.get('status')));
trace=[json.loads(l) for l in (base/'traceability-matrix.jsonl').read_text().splitlines() if l.strip()]; ids=set(expected); missing=[(r.get('contract_clause'),p) for r in trace for p in r.get('proofs',[]) if p not in ids and not str(p).startswith('WAIVER-')];
if missing: bad.append(('trace-missing',missing));
print('\n'.join(report)); raise SystemExit(repr(bad) if bad else 0)"
```

Observed output:

```text
proof-obligations.jsonl: VERUS-EXIT-001,TEST-EXIT-001,STATIC-EXIT-001,TEST-DIAGNOSTICS-001,TEST-STRUCTURED-001,TEST-POSTCARD-001,FUZZ-POSTCARD-001,RECON-CHILD-001,MATRIX-COMMAND-001
proof-obligations.planned.jsonl: VERUS-EXIT-001,TEST-EXIT-001,STATIC-EXIT-001,TEST-DIAGNOSTICS-001,TEST-STRUCTURED-001,TEST-POSTCARD-001,FUZZ-POSTCARD-001,RECON-CHILD-001,MATRIX-COMMAND-001
```

Decision: PASS. Both ledgers contain exactly the current 9 IDs in order, no row uses `PASS` status, no active row contains placeholder command markers, and traceability proof references resolve to current IDs or explicit `WAIVER-*` rationale rows.

## Obligation Decisions

### VERUS-EXIT-001

Artifact reviewed: `verification/verus/diagnostic_envelope_verus.rs`.

Relevant source facts:

- Lines 14-24 model exactly nine public exit-code variants and no `DomainError` variant.
- Lines 26-42 map discriminants to `0..=8` and define `spec_exit_code_in_range_0_to_8`.
- Lines 44-58 prove `lemma_exit_code_range_0_to_8` by exhaustive match.

Command rerun:

```bash
verus "verification/verus/diagnostic_envelope_verus.rs"
```

Observed output:

```text
verification results:: 4 verified, 0 errors
```

Decision: PASS.

### TEST-EXIT-001

Artifact reviewed: `crates/velvet_ballastics/src/exit_code.rs`.

Relevant source facts:

- Lines 12-31 define `CliExitCode` variants `0..=8`; no public code 9 exists.
- Lines 61-72 assert exact discriminants through `ReplayDivergence = 8`.
- Lines 141-156 assert all public values are `<= 8`.

Command rerun:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics exit_code --all-features
```

Observed output excerpt:

```text
running 9 tests
test exit_code::tests::all_variants_are_distinct ... ok
test exit_code::tests::all_variants_are_public_range_0_to_8 ... ok
test exit_code::tests::from_journal_error_maps_to_storage_error ... ok
test exit_code::tests::discriminant_values_match_spec ... ok
test exit_code::tests::from_core_error_maps_to_runtime_failed ... ok
test exit_code::tests::from_cli_exit_code_to_exit_code ... ok
test mode_activation_tests::cli_exit_code_all_9_variants_distinct ... ok
test mode_activation_tests::mode_error_all_variants_have_distinct_exit_codes ... ok
test mode_activation_tests::parse_error_unknown_command_exit_code_is_1 ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 164 filtered out; finished in 0.00s
```

Additional matching integration tests also passed: `cli_run_invalid_workflow_returns_error_exit_code`, `bdd_yaml_parse_exit_code_is_validation_failed`, `bdd_inv001_exit_code_stable_across_formats_on_error`, and `bdd_format_parity_exit_code_identical_across_formats`.

Decision: PASS.

### STATIC-EXIT-001

Artifacts reviewed: `crates/velvet_ballastics/src/exit_code.rs` and `verification/verus/diagnostic_envelope_verus.rs`.

Command rerun:

```bash
if rg -n "DomainError\s*=\s*9|ExitCode::from\(9u8\)|0_to_9|<= 9" "crates/velvet_ballastics/src/exit_code.rs" "verification/verus/diagnostic_envelope_verus.rs"; then exit 2; else code=$?; test "$code" -eq 1; fi
```

Observed output: none. Exit status: 0.

Reviewer note: an earlier wrapper used `status` as a zsh variable name and failed with `zsh:1: read-only variable: status`; the corrected rerun above passed and is the evidence for this obligation.

Decision: PASS. No active public `DomainError = 9`, `ExitCode::from(9u8)`, stale `0_to_9`, or stale public `<= 9` proof residue remains in the required source/proof files.

### TEST-DIAGNOSTICS-001

Command rerun:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics parse_error_unknown_command_exit_code_is_1 --all-features
```

Observed output excerpt:

```text
running 1 test
test mode_activation_tests::parse_error_unknown_command_exit_code_is_1 ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 172 filtered out; finished in 0.00s
```

Decision: PASS.

### TEST-STRUCTURED-001

Command rerun:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics bdd_format_parity_exit_code_identical_across_formats --all-features
```

Observed output excerpt:

```text
running 1 test
test bdd_format_parity_exit_code_identical_across_formats ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.00s
```

Decision: PASS.

### TEST-POSTCARD-001

Command rerun:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard
```

Observed output:

```text
running 8 tests
test emitter::binary::tests::tests::postcard_rejects_bad_crc ... ok
test emitter::binary::tests::tests::encode_decode_postcard_roundtrip ... ok
test emitter::binary::tests::tests::postcard_rejects_bad_magic ... ok
test emitter::binary::tests::tests::postcard_rejects_bad_payload_digest ... ok
test emitter::binary::tests::tests::postcard_rejects_payload_too_large ... ok
test emitter::binary::tests::tests::postcard_rejects_wrong_kind ... ok
test emitter::binary::tests::tests::postcard_rejects_unsupported_version ... ok
test emitter::binary::tests::tests::postcard_rejects_old_version ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.00s
```

Decision: PASS.

### FUZZ-POSTCARD-001

Artifacts reviewed: `fuzz/Cargo.toml`, `fuzz/fuzz_targets.rs`, `fuzz/src/lib.rs`, and `fuzz/src/bin/vb_ui_model_postcard_decode.rs`.

Relevant source facts:

- `fuzz/src/lib.rs:1482-1511` decodes `vb_ui_model::envelope::OutputEnvelope` from arbitrary postcard bytes and checks schema/kind/payload/diagnostic invariants for structurally valid envelopes.
- `fuzz/src/bin/vb_ui_model_postcard_decode.rs:3-31` provides the stdin-compatible executable wrapper.

Stdin harness rerun:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo run --manifest-path fuzz/Cargo.toml --features fuzz --bin vb_ui_model_postcard_decode -- < /dev/null
```

Observed output excerpt:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
Running `target/debug/vb_ui_model_postcard_decode`
```

Explicit GNU cargo-fuzz rerun:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1
```

Observed output excerpt:

```text
Finished `release` profile [optimized + debuginfo] target(s) in 0.09s
Finished `release` profile [optimized + debuginfo] target(s) in 0.07s
Running `target/x86_64-unknown-linux-gnu/release/vb_ui_model_postcard_decode -artifact_prefix=/home/lewis/src/vb-qi37-13-r2/fuzz/artifacts/vb_ui_model_postcard_decode/ -runs=1 /home/lewis/src/vb-qi37-13-r2/fuzz/corpus/vb_ui_model_postcard_decode`
```

Decision: PASS for the planned smoke obligation. The default musl/ASAN issue remains only a `CANDIDATE_NOT_APPROVED` tooling waiver and is not used to discharge this obligation.

### RECON-CHILD-001

Command rerun:

```bash
python3 -c "from pathlib import Path; base=Path('.beads/vb-qi37.13'); texts={n:(base/n).read_text() for n in ['proof-evidence.md','proof-review.md','contract-verification-review.md']}; required=['PO-VERUS-EXIT-001: PASS','PO-TEST-EXIT-001 evidence check: PASS','PO-STATIC-EXIT-001 evidence check: PASS','PO-POSTCARD-ROUTE-001: PASS','STATUS: APPROVED','STATUS: REJECTED','vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1']; missing=[s for s in required if not any(s in t for t in texts.values())]; raise SystemExit('missing child evidence markers: '+repr(missing) if missing else 0)"
```

Observed output: none. Exit status: 0.

Decision: PASS. This is accepted only as marker reconciliation. It is not treated as independent proof of child correctness; the current primary obligation ledger, command matrix, traceability, and rerun commands above provide the non-laundered evidence.

### MATRIX-COMMAND-001

Command rerun:

```bash
python3 -c "import json; from pathlib import Path; base=Path('.beads/vb-qi37.13'); obs=[json.loads(l) for l in (base/'proof-obligations.jsonl').read_text().splitlines() if l.strip()]; ids={o['id'] for o in obs}; markers=['T'+'ODO','UNKN'+'OWN','DISCOVER'+'_','REGISTER'+'_AND_RUN']; bad=[o['id'] for o in obs if o.get('status')=='PASS' or any(m in o.get('command','') for m in markers)]; rows=[json.loads(l) for l in (base/'traceability-matrix.jsonl').read_text().splitlines() if l.strip()]; missing=[(r.get('contract_clause'),p) for r in rows for p in r.get('proofs',[]) if p not in ids and not str(p).startswith('WAIVER-')]; raise SystemExit('bad='+repr(bad)+' missing='+repr(missing) if bad or missing else 0)"
```

Observed output: none. Exit status: 0.

Decision: PASS.

## Adversarial Checks

- No public exit 9 remains in the reviewed active public exit-code source.
- No stale `0_to_9` or public `<= 9` proof remains in the reviewed active Verus proof artifact.
- All planned IDs are accounted in both current obligation ledgers.
- No current obligation command uses placeholder markers.
- The child evidence reconciliation row is not allowed to launder evidence: approval rests on raw rerun command evidence plus the current command matrix checks, not on child summaries alone.
- The candidate cargo-fuzz musl/ASAN waiver is not approved and is not used as PASS evidence.

## Routing

State 6 proof review is approved. No proof repair is required.
