# Canonical answer help

The public global help and scoped answer help must show the existing canonical command `answer <run_id> --slot <N> --value <file> --db <path> [--emit text|yaml|postcard]`, matching MASTER section69 and the admitted parser/metadata selector route recovered in48. Help must not direct users exclusively to the older step selector.

This changes only the answer line in the shared HELP string. Every other help byte remains unchanged. Parser selector ranges, legacy compatibility behavior, pending Ask authority, command metadata, output format behavior, error precedence, runtime/storage wire formats and all existing tests stay unchanged. No old webhook/status-DB claims, dependencies, schema changes, snapshots or protected gate edits are part of this repair.

The boundary is the real CLI process writing usage to stdout. Global `help`, `answer --help` and `help answer` must succeed with empty stderr and show the exact canonical answer usage. The focused registered CLI target must first fail against unchanged production and then pass after the single-line repair. Existing parser-backed answer metadata tests remain independent authority; no implementation helper is used to manufacture expected usage. Source/fmt/integrity checks use the literal94a base, and independent CODE/TEST review precedes parent landing.

Original public evidence is retained in ignored help63 artifacts: the actual scoped answer usage contains only `--step` and `--value-file`, exit0, empty stderr. This is a user-facing usage defect; no runtime admission failure or security claim is inferred. CPU/cloud only, GPUs prohibited, benchmarks skipped.
