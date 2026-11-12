# Test-integrity justification — check-test-integrity.rs source-length split (vb-srclen)

Base: `c9097bdfde6e32aee88ccdd50dd7e557367153d0`. Branch `vb/split-check-test-integrity`,
commit `61acdf57`. Ledger bead `vb-srclen`.

## What changed

`scripts/check-test-integrity.rs` (2480 lines, production, limit 300) split by pure
relocation into a 215-line shell plus 13 part modules, each under 300 lines. The
source-length ledger row for the file was removed. No behavior changed: every moved
cluster is byte-identical except module headers (`use` lines), `mod` declarations,
`pub(crate)` visibility on cross-module items, and rustfmt re-wraps of the five
signatures the added `pub(crate)` prefix pushed past 100 columns
(`changed_files`, `resolve_base_blob`, `path_diff_bytes`, `expect_findings`,
`justification_fixture`). The seven rustfmt hunks in the moved code are
byte-identical to the drift in the base file.

| Original lines | Content | New location |
| --- | --- | --- |
| 1-55 | imports, shared types, `JUSTIFICATIONS_REL` | shell (imports trimmed to the two still used) |
| 60-124 | command helpers (`tool_output*`, `git_*`, `jj_*`) | `check_test_integrity_commands.rs` |
| 130-179 | `is_elf`, `resolve_real_binary`, `git_cmd` | `check_test_integrity_binary.rs` |
| 185-258 | `root_dir`, `default_base`, `validate_base_revision`, `resolve_base_hash` | `check_test_integrity_roots.rs` |
| 264-377 | path classification + text-signal needles | `check_test_integrity_paths.rs` |
| 383-649 | changed-file / rename-pair enumeration, blob resolution, `path_diff_bytes` | `check_test_integrity_diff.rs` |
| 651-922 | `scan_diff`, `scan_changed_entry`, rename-pair scanning | `check_test_integrity_scan.rs` |
| 924-949 | `collect_findings` | shell |
| 955-1175 | justification parsing + `JustificationRecord` impl | `check_test_integrity_justifications.rs` |
| 1177-1252 | `load_justifications`, `apply_justifications` | `check_test_integrity_apply.rs` |
| 1258-1279 | `check` | shell |
| 1285-1449, 1892-1946 | `Fixture`, test helpers, `BASE_TEXT`, `large_test_*`, justification fixtures | `check_test_integrity_fixture.rs` |
| 1451-1665 | core diff self-test cases | `check_test_integrity_cases.rs` |
| 1667-1886 | relocation self-test cases | `check_test_integrity_cases_rename.rs` |
| 1948-2239 | `run_justification_tests` | `check_test_integrity_justification_tests.rs` |
| 2241-2404 | justification bypass + lifecycle tests | `check_test_integrity_bypass_tests.rs` |
| 2410-2441 | `self_test` entry | shell |
| 2447-2480 | CLI (`argument_value`, `run`, `main`) | shell |

## Why the two findings are relocations, not deletions

The gate scans the shell file's diff in isolation, so code that leaves the shell
reads as removed. Both findings count only lines that moved verbatim into the part
modules:

- `DeletedTestDeclaration`, `removed_declarations=16 added_declarations=0`: the
  16 removed test-declaration signals are the `#[test]` / `fn test_` / `_test(`
  needles in `has_test_decl` (now `check_test_integrity_paths.rs`) and the
  `behavior_test` string literals in the self-test fixtures (now
  `check_test_integrity_fixture.rs`).
- `WeakenedAssertion`, `removed_exact=13 added_exact=0 added_weak=0`: the 13
  removed exact-assertion signals are the `assert_eq!` / `assert_ne!` / etc.
  needles in `has_exact_assertion` (now `check_test_integrity_paths.rs`) and the
  `assert_eq!` string literals in the self-test fixtures and cases (now
  `check_test_integrity_fixture.rs`, `check_test_integrity_cases.rs`,
  `check_test_integrity_bypass_tests.rs`).

No test case, assertion, or fixture was deleted or weakened. The replacement
coverage listed in the justification records is the six part modules that hold
the moved test signals (`paths`, `fixture`, `cases`, `cases_rename`,
`justification_tests`, `bypass_tests`). The other seven part modules contain no
test signals (verified by grep over the full needle set); `cases_rename` and
`justification_tests` hold moved `run_*` case functions whose literals live in
`fixture`.

## Verification (first-hand, branch vs base `c9097bdf`)

- `bash scripts/check-test-integrity.sh --self-test`: exit 0, 33 PASS, 0 FAIL,
  output byte-identical to the base run.
- Byte-identity: a script re-read each written file, stripped its header and
  `pub(crate)` markers, and compared the body line-for-line against the
  original slices; all 13 parts and the shell body verified.
- `rustfmt --edition 2024 --check` on the 14 files: only the 7 pre-existing
  drift hunks, identical to the base file's drift (see PR body).
- The two gate findings above are the only test-integrity findings on the
  branch diff; these records justify exactly those two, bound to this base.

These records are bound to base `c9097bdf...`. They go inactive when main
advances past this base, by design: once the split is committed, later bases
have no such diff and no findings arise.
