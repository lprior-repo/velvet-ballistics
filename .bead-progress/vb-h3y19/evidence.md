# vb-h3y19 — product-positioning-lint evidence

Bead: `vb-h3y19` — product-positioning-lint (P2, IN_PROGRESS)

## Master quote (verbatim)

`velvet-ballistics-MASTER.md:29`:

> Publicly, velvet-ballistics must not be described as a generic DAG
> runner, low-code graph editor, YAML-as-programming framework, Airflow
> replacement, or Temporal clone. Those frames hide the actual wedge and
> invite false comparisons.

## Files delivered

| Path | Bytes | Mode |
|------|------:|------|
| `scripts/check-product-positioning.sh`  |  1958 | `-rwxr-xr-x` |
| `scripts/check-product-positioning.rs`  | 11623 | `-rw-r--r--` |
| `scripts/test-check-product-positioning.sh` | 3065 | `-rwxr-xr-x` |
| `fixtures/product-positioning/positive.md`  |  1069 | `-rw-r--r--` |
| `fixtures/product-positioning/negative.md`  |   719 | `-rw-r--r--` |
| `.bead-progress/vb-h3y19/evidence.md`       |  this | `-rw-r--r--` |

Plus a one-line disclaimer-block wrap on
`docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:15-17` so its existing
"the product is not a …" paragraph is recognized as a legitimate
disclaimer (see §"Repo hygiene change" below).

## Repo hygiene change

`docs/adr/v1/ADR-001-v1-backend-ir-north-star.md` already used a
"the product is not a …" sentence that contains four banned phrases
(`generic DAG runner`, `low-code graph editor`, `Airflow replacement`,
`Temporal clone`). The new scanner treats that as a legitimate
"disclaimered" block, not an active violation. The block markers
`<!-- position-disclaimer -->` … `<!-- /position-disclaimer -->` were
added around that single sentence. No other prose changed.

## Raw `ls -la` output (6 deliverable files)

```
-rw-r--r-- 1 lewis lewis   719 Jun 14 15:23 fixtures/product-positioning/negative.md
-rw-r--r-- 1 lewis lewis  1069 Jun 14 15:23 fixtures/product-positioning/positive.md
-rw-r--r-- 1 lewis lewis 11623 Jun 14 15:23 scripts/check-product-positioning.rs
-rwxr-xr-x 1 lewis lewis  1958 Jun 14 15:23 scripts/check-product-positioning.sh
-rwxr-xr-x 1 lewis lewis  3065 Jun 14 15:23 scripts/test-check-product-positioning.sh
-rw-r--r-- 1 lewis lewis   ~  Jun 14 15:24 .bead-progress/vb-h3y19/evidence.md
```

## Verification command outputs

### 1. `bash scripts/test-check-product-positioning.sh` → exit 0

```
[1/3] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/3] negative fixture must FAIL (exit 1, file:line finding)
  ok: exit 1 with file:line finding
[3/3] real repository scan must PASS (exit 0, no active residue)
  ok: exit 0
  ok: summary reports active=0
  ok: no POSITIONING line in output
self-test PASSED
exit=0
```

### 2. `bash scripts/check-product-positioning.sh` (full repo scan) → exit 0

```
docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:16: disclaimered: generic dag runner: The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.
docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:16: disclaimered: low-code graph editor: The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.
docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:16: disclaimered: airflow replacement: The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.
docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:16: disclaimered: temporal clone: The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.
summary: active=0 allowlisted=0 disclaimered=4 files_scanned=89
exit=0
```

(`exit=0` confirmed with `bash scripts/check-product-positioning.sh >/dev/null 2>&1; echo $?` → `0`.)

### 3a. `bash scripts/check-product-positioning.sh fixtures/product-positioning/positive.md` → exit 0

```
summary: active=0 allowlisted=0 disclaimered=0 files_scanned=1
exit=0
```

### 3b. `bash scripts/check-product-positioning.sh fixtures/product-positioning/negative.md` → exit 1

```
fixtures/product-positioning/negative.md:5: POSITIONING: airflow replacement: airflow replacement on the market, which is exactly the framing master
fixtures/product-positioning/negative.md:11: POSITIONING: airflow replacement: velvet-ballistics is the fastest airflow replacement on the market.
fixtures/product-positioning/negative.md:12: POSITIONING: temporal alternative: It also doubles as a temporal alternative for legacy migrations.
summary: active=3 allowlisted=0 disclaimered=0 files_scanned=1
exit=1
```

### 4. `rtk rg -n '\.unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!|dbg!|unsafe[^_]' scripts/check-product-positioning.rs` → empty (exit 1, no match)

```
exit=1 (1=no match = PASS)
```

The scanner source contains zero forbidden constructs:
no `.unwrap()`, no `.expect(`, no `panic!`, no `todo!`, no `unimplemented!`,
no `dbg!`, and no `unsafe` followed by a non-`_` character.
`#![forbid(unsafe_code)]` and the `clippy::unwrap_used / expect_used /
panic / todo / unimplemented / dbg_macro` `deny` group are the only
constructs that mention those tokens, and they all use the underscore
spelling required by the regex.

## Exit-code summary table

| Command | Required exit | Observed exit |
|---|---:|---:|
| `bash scripts/test-check-product-positioning.sh` | 0 | **0** |
| `bash scripts/check-product-positioning.sh` | 0 | **0** |
| `bash scripts/check-product-positioning.sh fixtures/product-positioning/positive.md` | 0 | **0** |
| `bash scripts/check-product-positioning.sh fixtures/product-positioning/negative.md` | 1 | **1** |
| `rtk rg -n '<forbidden>' scripts/check-product-positioning.rs` | empty | **empty** |

## Implementation contract

- **Holzman compliance**: `#![forbid(unsafe_code)]` + `clippy::unwrap_used`,
  `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro` all `deny`.
  No `unwrap()` / `expect()` / `panic!` / `todo!` / `unimplemented!` / `dbg!`
  in source. Counters use `saturating_add`.
- **Scan surface** (default): `README.md`, `docs/**/*.md`,
  `crates/**/README.md`, `crates/vb_cli/**/*.md`. Recursive top-down.
- **Self-skip basenames**: `velvet-ballistics-MASTER.md`, `CHANGELOG.md`,
  `HISTORY.md`, `MIGRATION.md`.
- **Self-skip directories**: `target/`, `node_modules/`, `.bead-progress/`,
  `.evidence/`.
- **Per-line allowlist**: `<!-- ALLOW_HISTORICAL: <reason> -->` on the
  same line as a banned phrase suppresses that line; reported as
  `allowlisted: <reason>: <line>`.
- **Block allowlist**: `<!-- position-disclaimer -->` … `<!-- /position-disclaimer -->`
  suppresses every match inside; reported as `disclaimered: <phrase>: <line>`.
- **Output format**: `<rel>:N: POSITIONING: <phrase>: <line>` to stderr.
  Summary line: `summary: active=N allowlisted=M disclaimered=K files_scanned=J`.
  Exit 0 iff `active == 0`.

## Suggested `bd close` reason

```bash
bd close vb-h3y19 --reason "Implemented product-positioning-lint per velvet-ballistics-MASTER.md:29. Delivered: scripts/check-product-positioning.{sh,rs} (Holzman-compliant scanner, no unsafe/unwrap/expect/panic/todo/unimplemented/dbg), scripts/test-check-product-positioning.sh (3-step self-test, exit 0), fixtures/product-positioning/{positive,negative}.md. Negative fixture fails with file:line findings (3 active matches). Real repo scan passes (active=0, 4 disclaimered lines in ADR-001 wrapped in <!-- position-disclaimer -->) exit 0. Forbidden-construct rg check is empty. Evidence: .bead-progress/vb-h3y19/evidence.md."
```
