bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 4
updated_at: 2026-05-14T23:05:00Z
attempt: 2-of-7

# Proof Plan Review Input

## Skill Citation

Proof-planner skill `version=1.0.1` requires planning artifacts only, exact verifier commands, traceability for every obligation, and explicit waiver/tooling rows for skipped applicable verifier lanes. This review input is therefore limited to `.beads/vb-qi37.13/` planning artifacts and does not write proof, production, or test code.

## Clauses Under Review

- `POST-001` and `INV-001`: every public CLI process exit status is in `0..=8`.
- `POST-002` and `ERR-009`: no public CLI path emits `DomainError = 9` or any other code outside `0..=8`.
- `PRE-001`, `POST-004`, and `INV-004`: unsupported commands/output/emit modes fail closed with typed validation diagnostics and exit code `1`.
- `POST-003` and `INV-005`: structured operator output keeps stable `schema_version` and `kind` fields.
- `PRE-002`, `INV-003`, and `ERR-010` through `ERR-016`: postcard decode rejects malformed envelopes before payload exposure.
- `POST-005`: postcard output/decode evidence is integrated through repository tests/fuzz with the pinned GNU cargo-fuzz command.
- `POST-006` and `INV-006`: State 3/5/6 child evidence and command matrices reconcile with executable commands and no planned pass status.

## Planned Primary Obligations

The planned JSONL must contain these primary IDs exactly once:

- `VERUS-EXIT-001`
- `TEST-EXIT-001`
- `STATIC-EXIT-001`
- `TEST-DIAGNOSTICS-001`
- `TEST-STRUCTURED-001`
- `TEST-POSTCARD-001`
- `FUZZ-POSTCARD-001`
- `RECON-CHILD-001`
- `MATRIX-COMMAND-001`

## Exact Command Matrix

`VERUS-EXIT-001`:

```bash
verus verification/verus/diagnostic_envelope_verus.rs
```

`TEST-EXIT-001`:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics exit_code --all-features
```

`STATIC-EXIT-001`:

```bash
rg -n "DomainError\s*=\s*9|ExitCode::from\(9u8\)|0_to_9|<= 9" crates/velvet_ballistics/src/exit_code.rs verification/verus/diagnostic_envelope_verus.rs
```

`TEST-DIAGNOSTICS-001`:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics parse_error_unknown_command_exit_code_is_1 --all-features
```

`TEST-STRUCTURED-001`:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics bdd_format_parity_exit_code_identical_across_formats --all-features
```

`TEST-POSTCARD-001`:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard
```

`FUZZ-POSTCARD-001`:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1
```

`RECON-CHILD-001`:

```bash
python3 -c "from pathlib import Path; base=Path('.beads/vb-qi37.13'); texts={n:(base/n).read_text() for n in ['proof-evidence.md','proof-review.md','contract-verification-review.md']}; required=['PO-VERUS-EXIT-001: PASS','PO-TEST-EXIT-001 evidence check: PASS','PO-STATIC-EXIT-001 evidence check: PASS','PO-POSTCARD-ROUTE-001: PASS','STATUS: APPROVED','STATUS: REJECTED','vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1']; missing=[s for s in required if not any(s in t for t in texts.values())]; raise SystemExit('missing child evidence markers: '+repr(missing) if missing else 0)"
```

`MATRIX-COMMAND-001`:

```bash
python3 -c "import json; from pathlib import Path; base=Path('.beads/vb-qi37.13'); obs=[json.loads(l) for l in (base/'proof-obligations.jsonl').read_text().splitlines() if l.strip()]; ids={o['id'] for o in obs}; markers=['T'+'ODO','UNKN'+'OWN','DISCOVER'+'_','REGISTER'+'_AND_RUN']; bad=[o['id'] for o in obs if o.get('status')=='PASS' or any(m in o.get('command','') for m in markers)]; rows=[json.loads(l) for l in (base/'traceability-matrix.jsonl').read_text().splitlines() if l.strip()]; missing=[(r.get('contract_clause'),p) for r in rows for p in r.get('proofs',[]) if p not in ids and not str(p).startswith('WAIVER-')]; raise SystemExit('bad='+repr(bad)+' missing='+repr(missing) if bad or missing else 0)"
```

## Tooling Note

The default cargo-fuzz musl/ASAN failure is not a planned proof command and must not be required as passing evidence. It remains a waiver candidate with owner `formal-verifier` only if a later gate rejects the pinned GNU command.

## Reviewer Questions

- Are the nine repaired primary obligation IDs sufficient and non-duplicated?
- Does the pinned GNU cargo-fuzz command satisfy `REQ-POSTCARD-PROOF` for State 4 planning?
- Are `RECON-CHILD-001` and `MATRIX-COMMAND-001` strong enough to catch child evidence drift and placeholder command regressions?
