# Test-integrity justification — check_test_ownership.rs source-length split (vb-srclen)

Base: `f8cc55ef6cb83eb374ecf4aa407914afe59c9e9c`. Branch `vb/split-check-test-ownership`,
commit `62ef2f38`. Ledger bead `vb-srclen`.

## What changed

`scripts/check_test_ownership.rs` (302 lines, production, limit 300) split by pure
relocation into a 269-line shell plus `scripts/check_test_ownership_scope.rs`
(37 lines). No behavior changed: every moved cluster is byte-identical except
the `#[path]` mod declaration, the private `use` re-import, the two `pub(crate)`
visibility markers, and one blank line collapsed at the cut point (the shell head
already ended with a blank line). The two pre-existing rustfmt drift hunks are in
the shell's `self_test`, untouched by this split; the moved code is fmt-clean.
`.moon/tasks/all.yml` lists the new part file in the panic-surface and check
tasks, next to the shell.

| Original lines | Content | New location |
| --- | --- | --- |
| 17-32 | `closing_brace` | `check_test_ownership_scope.rs` |
| 34-51 | `leading_file_cfg_test` | `check_test_ownership_scope.rs` |

## Why the finding is a relocation, not a deletion

The gate scans the shell file's diff in isolation, so code that leaves the shell
reads as removed. The single finding counts one removed test-declaration signal:

- `DeletedTestDeclaration`, `removed_declarations=1 added_declarations=0`: the
  removed signal is the `fn leading_file_cfg_test(` declaration line, which
  matches the gate's `_test(` needle in `has_test_decl`. The function was not
  deleted: it moved verbatim into `check_test_ownership_scope.rs` (the
  replacement coverage in the justification record) and is still called from
  the same three sites in `test_scopes` via the shell's private re-import.
  `closing_brace` carries no test signal and moved with it. No test case,
  assertion, or fixture was deleted or weakened; no `WeakenedAssertion`
  finding arose (removed_exact=0).

## Verification (first-hand, branch vs base `f8cc55ef`)

- `bash scripts/check-test-integrity.sh --self-test`: exit 0 on both, output
  byte-identical (122 SelfTest PASS lines, 0 FAIL), including
  `SelfTest:ownership-lexer: PASS`.
- Byte-identity: a script re-read each written file, stripped its header and
  `pub(crate)` markers, and compared the body line-for-line against the
  original slices; the shell body and the part body both verified, re-verified
  after a `git stash` round-trip.
- `rustfmt --edition 2024 --check` on the two files: only the 2 pre-existing
  drift hunks, identical to the base file's drift, shifted by exactly 33 lines
  (185 to 152, 198 to 165).
- `bash scripts/check-panic-surface.sh`: exit 2 on BOTH base and branch, logs
  byte-identical (pre-existing violations in `tests_bound_recovery.rs`,
  untouched by this split).
- Source-length gate: production over_limit 1 to 0; the ownership FAIL is gone;
  the 4 hot-function FAILs are unchanged (pre-existing, function refactors,
  out of lane-B scope).
- The gate finding above is the only test-integrity finding on the branch
  diff; this record justifies exactly it, bound to this base.

These records are bound to base `f8cc55ef...`. They go inactive when main
advances past this base, by design: once the split is committed, later bases
have no such diff and no findings arise.
