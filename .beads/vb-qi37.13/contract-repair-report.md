# Contract Repair Report

STATUS: REPAIRED

## Scope

- Worktree: `/home/lewis/src/vb-qi37-13-r2` only.
- Repaired State 3 contract-layer artifacts after State 6 contract-verification rejection.
- Cited startup rules: `/home/lewis/.claude/skills/rust-contract/SKILL.md` and winning `/home/lewis/.agents/skills/rust-contract/SKILL.md`, both version `2.6.0`; key enforced clauses were `no_invented_formal_targets`, `token_efficient_obligations`, valid JSONL, exact commands, planned contract-time statuses, and complete traceability.

## Changes

- `contract.md`: strengthened, not weakened, by freezing child evidence reconciliation (`POST-006`), command matrix invariant (`INV-006`), and postcard error variants `ERR-010` through `ERR-016`.
- `proof-obligations.jsonl`: replaced placeholder commands with exact executable commands; added structured-output, diagnostics, postcard unit, child reconciliation, and command-matrix obligations. All statuses remain `planned`; no row claims `PASS`.
- `proof-obligations.planned.jsonl`: back-propagated exact Verus/static/test/fuzz commands so State 4 planned rows no longer contain discovery macros.
- `traceability-matrix.jsonl`: expanded from 4 rows to 33 rows covering all preconditions, postconditions, invariants, CLI exit-code error variants, postcard error variants, child evidence reconciliation, command matrix, and TLA/Lean waiver rationale rows.

## Primary exact commands pinned

- `verus verification/verus/diagnostic_envelope_verus.rs`
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics exit_code --all-features`
- `rg -n "DomainError\\s*=\\s*9|ExitCode::from\\(9u8\\)|0_to_9|<= 9" crates/velvet_ballastics/src/exit_code.rs verification/verus/diagnostic_envelope_verus.rs`
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics parse_error_unknown_command_exit_code_is_1 --all-features`
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics bdd_format_parity_exit_code_identical_across_formats --all-features`
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard`
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1`

## Coverage summary

- Preconditions: `PRE-001` through `PRE-003` covered.
- Postconditions: `POST-001` through `POST-006` covered.
- Invariants: `INV-001` through `INV-006` covered.
- CLI exit taxonomy: `ERR-001` through `ERR-009` covered.
- Postcard error variants: `ERR-010` through `ERR-016` covered.
- Waiver rationale rows: `WAIVER-TLA-001`, `WAIVER-LEAN-001` represented for review traceability.
- Traceability row count: 33.

## Validation commands

Run from `/home/lewis/src/vb-qi37-13-r2`:

```bash
python3 -m json.tool .beads/vb-qi37.13/proof-obligations.jsonl >/dev/null
```

The above whole-file command is not valid for JSONL; use linewise validation instead:

```bash
python3 -c "import json,pathlib; [json.loads(l) for p in ['.beads/vb-qi37.13/proof-obligations.jsonl','.beads/vb-qi37.13/proof-obligations.planned.jsonl','.beads/vb-qi37.13/traceability-matrix.jsonl'] for l in pathlib.Path(p).read_text().splitlines() if l.strip()]"
python3 -c "import json,pathlib; p=pathlib.Path('.beads/vb-qi37.13/proof-obligations.jsonl'); markers=['T'+'ODO','UNKN'+'OWN','DISCOVER'+'_','REGISTER'+'_AND_RUN']; rows=[json.loads(l) for l in p.read_text().splitlines() if l.strip()]; bad=[r['id'] for r in rows if r.get('status')=='PASS' or any(m in r.get('command','') for m in markers)]; raise SystemExit(bad if bad else 0)"
```
