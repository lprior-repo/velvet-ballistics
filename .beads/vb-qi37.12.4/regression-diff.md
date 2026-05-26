# Regression Diff: vb-qi37.12.4

## Changed Areas

- Direct ignored-fallible-results scanner and verify-standard gate wiring.
- Explicit handling of previously ignored results in runtime tests/models, IPC helpers/tests, storage process lock, UI test/support code, and CLI output/input helpers.

## Regression Evidence

- Direct gate passes with `NoViolationFound`.
- Affected non-excluded packages pass: `vb_runtime`, `vb_ipc`, `vb_storage`, `velvet_ballistics` serial.
- `moon run :verify-standard` passes including Kani standard harnesses.

## Deferred Global Debt

- Excluded `crates/vb_ui` manifest test fails on unrelated `JournalEvent` initializer compile errors requiring `attempt` fields. This is not caused by the ignored-result repair and is not part of the current bead acceptance scope.
