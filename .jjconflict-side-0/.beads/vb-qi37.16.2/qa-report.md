# State 9 QA Report — vb-qi37.16.2

Updated: 2026-05-11

## Automated QA Evidence

`moon ci` was executed after State 8 repairs and passed all resolved CI tasks:

```text
Tasks: 19 completed (2 cached)
Time: 2m 59s 716ms
Output: /home/lewis/.local/share/opencode/tool-output/tool_e19a150f1001I6zN6ZgIuCrYGZ
```

Included evidence from the CI run:

- `fmt`: PASS
- `lint-src`: PASS
- `check`: PASS
- `test`: PASS — nextest ran 8014 tests, 8014 passed, 0 skipped
- `miri`: PASS — three scoped Miri probes passed
- `coverage`: PASS — report saved to `target/llvm-cov/lcov.info`
- `mutants-smoke`: PASS — 1 mutant tested, 1 caught
- `feature-powerset`: PASS — 29/29 package/feature checks completed
- `bench-build`, `doc`, `doc-test`, `hardened-build`, `maxperf`, `maxperf-native`: PASS

## State 9 Result

STATUS: APPROVED
