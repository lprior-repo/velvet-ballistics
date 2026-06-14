# vb-h3y19 — product-positioning-lint evidence

Bead: `vb-h3y19` — product-positioning-lint (P2, IN_PROGRESS)

## Modified files

- `scripts/check-product-positioning.rs`
- `scripts/check-product-positioning.sh`
- `scripts/test-check-product-positioning.sh`
- `fixtures/product-positioning/negative.md`

## Change summary

- Unicode NFKC + zero-width stripping + separator folding before phrase matching.
- Shell wrapper now compiles through a temp cargo manifest so it can use `unicode-normalization`.
- Disclaimer blocks now require negation markers and fail closed on unclosed EOF.
- Default scan surface now includes root `*.md` and `fuzz/*.md`.
- Negative fixture and self-test now cover all banned phrases and bypass shapes.

## Power-of-Ten / zero-panic impact

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg!` added.
- No unchecked indexing or ignored fallible results added.
- Balanced disclaimer handling is explicit and fail-closed.

## Verification

### 1. `bash scripts/test-check-product-positioning.sh`

Observed exit: `0`

```text
[1/7] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/7] negative fixture must FAIL and exercise every banned phrase
  ok: exit 1 with file:line findings
  ok: every banned phrase category appeared
[3/7] disclaimer-spam bypass must FAIL with active findings
  ok: exit 1 with active findings
[4/7] inline hyphen/underscore bypass must FAIL
  ok: exit 1 with active findings
[5/7] Unicode lookalike bypass must FAIL
  ok: exit 1 with active findings
[6/7] unclosed disclaimer block must FAIL hard
  ok: exit 2 scan error
[7/7] real repository scan must PASS (exit 0, no active residue)
  ok: exit 0
  ok: summary reports active=0
  ok: no POSITIONING line in output
self-test PASSED
```

### 2. `bash scripts/check-product-positioning.sh`

Observed exit: `0`

```text
docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:16: disclaimered: generic dag runner: The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.
docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:16: disclaimered: low-code graph editor: The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.
docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:16: disclaimered: airflow replacement: The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.
docs/adr/v1/ADR-001-v1-backend-ir-north-star.md:16: disclaimered: temporal clone: The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.
summary: active=0 allowlisted=0 disclaimered=4 files_scanned=118
```

### 3. `bash scripts/check-product-positioning.sh fixtures/product-positioning/positive.md`

Observed exit: `0`

```text
summary: active=0 allowlisted=0 disclaimered=0 files_scanned=1
```

### 4. `bash scripts/check-product-positioning.sh fixtures/product-positioning/negative.md`

Observed exit: `1`

```text
fixtures/product-positioning/negative.md:9: POSITIONING: generic dag runner: - velvet-ballistics is the generic dag runner for local teams.
fixtures/product-positioning/negative.md:10: POSITIONING: low-code graph editor: - velvet-ballistics is the low-code graph editor your ops team wanted.
fixtures/product-positioning/negative.md:11: POSITIONING: yaml-as-programming: - velvet-ballistics is the yaml-as-programming framework for builders.
fixtures/product-positioning/negative.md:11: POSITIONING: yaml as programming: - velvet-ballistics is the yaml-as-programming framework for builders.
fixtures/product-positioning/negative.md:12: POSITIONING: yaml-as-programming: - velvet-ballistics is the yaml as programming framework for builders.
fixtures/product-positioning/negative.md:12: POSITIONING: yaml as programming: - velvet-ballistics is the yaml as programming framework for builders.
fixtures/product-positioning/negative.md:13: POSITIONING: airflow replacement: - velvet-ballistics is the airflow replacement for brittle scheduling.
fixtures/product-positioning/negative.md:14: POSITIONING: airflow alternative: - velvet-ballistics is the airflow alternative for brittle scheduling.
fixtures/product-positioning/negative.md:15: POSITIONING: temporal clone: - velvet-ballistics is the temporal clone for durable execution.
fixtures/product-positioning/negative.md:16: POSITIONING: temporal alternative: - velvet-ballistics is the temporal alternative for durable execution.
summary: active=10 allowlisted=0 disclaimered=0 files_scanned=1
```

### 5. synthetic disclaimer-spam bypass

Observed exit: `1`

```text
/tmp/tmp7glbpwmm.md:2: POSITIONING: generic dag runner: velvet-ballistics is a generic DAG runner.
/tmp/tmp7glbpwmm.md:3: POSITIONING: low-code graph editor: velvet-ballistics is a low-code graph editor.
/tmp/tmp7glbpwmm.md:4: POSITIONING: yaml-as-programming: velvet-ballistics is a yaml-as-programming framework.
/tmp/tmp7glbpwmm.md:4: POSITIONING: yaml as programming: velvet-ballistics is a yaml-as-programming framework.
/tmp/tmp7glbpwmm.md:5: POSITIONING: yaml-as-programming: velvet-ballistics is a yaml as programming framework.
/tmp/tmp7glbpwmm.md:5: POSITIONING: yaml as programming: velvet-ballistics is a yaml as programming framework.
/tmp/tmp7glbpwmm.md:6: POSITIONING: airflow replacement: velvet-ballistics is a airflow replacement.
/tmp/tmp7glbpwmm.md:7: POSITIONING: airflow alternative: velvet-ballistics is a airflow alternative.
/tmp/tmp7glbpwmm.md:8: POSITIONING: temporal clone: velvet-ballistics is a temporal clone.
/tmp/tmp7glbpwmm.md:9: POSITIONING: temporal alternative: velvet-ballistics is a temporal alternative.
summary: active=10 allowlisted=0 disclaimered=0 files_scanned=1
exit=1
```

### 6. synthetic inline hyphen/underscore bypass

Observed exit: `1`

```text
/tmp/tmpblti1ak6.md:1: POSITIONING: generic dag runner: generic_dag_runner
/tmp/tmpblti1ak6.md:2: POSITIONING: low-code graph editor: low-code-graph-editor
/tmp/tmpblti1ak6.md:3: POSITIONING: yaml-as-programming: yaml-as-programming
/tmp/tmpblti1ak6.md:3: POSITIONING: yaml as programming: yaml-as-programming
/tmp/tmpblti1ak6.md:4: POSITIONING: yaml-as-programming: yaml_as_programming
/tmp/tmpblti1ak6.md:4: POSITIONING: yaml as programming: yaml_as_programming
/tmp/tmpblti1ak6.md:5: POSITIONING: airflow replacement: airflow-replacement
/tmp/tmpblti1ak6.md:6: POSITIONING: airflow replacement: airflow_replacement
/tmp/tmpblti1ak6.md:7: POSITIONING: airflow alternative: airflow-alternative
/tmp/tmpblti1ak6.md:8: POSITIONING: temporal clone: temporal-clone
/tmp/tmpblti1ak6.md:9: POSITIONING: temporal clone: temporal_clone
/tmp/tmpblti1ak6.md:10: POSITIONING: temporal alternative: temporal-alternative
summary: active=12 allowlisted=0 disclaimered=0 files_scanned=1
exit=1
```

### 7. synthetic Unicode lookalike / zero-width bypass

Observed exit: `1`

```text
/tmp/tmp_f1wu6t1.md:1: POSITIONING: generic dag runner: ｇｅｎｅｒｉｃ＿ｄａｇ＿ｒｕｎｎｅｒ
/tmp/tmp_f1wu6t1.md:2: POSITIONING: low-code graph editor: ｌｏｗ－ｃｏｄｅ－ｇｒａｐｈ－ｅｄｉｔｏｒ
/tmp/tmp_f1wu6t1.md:3: POSITIONING: yaml-as-programming: ｙａｍｌ－ａｓ－ｐｒｏｇｒａｍｍｉｎｇ
/tmp/tmp_f1wu6t1.md:3: POSITIONING: yaml as programming: ｙａｍｌ－ａｓ－ｐｒｏｇｒａｍｍｉｎｇ
/tmp/tmp_f1wu6t1.md:4: POSITIONING: yaml-as-programming: ｙａｍｌ＿ａｓ＿ｐｒｏｇｒａｍｍｉｎｇ
/tmp/tmp_f1wu6t1.md:4: POSITIONING: yaml as programming: ｙａｍｌ＿ａｓ＿ｐｒｏｇｒａｍｍｉｎｇ
/tmp/tmp_f1wu6t1.md:5: POSITIONING: airflow replacement: ａｉｒｆｌｏｗ＿ｒｅｐｌａｃｅｍｅｎｔ
/tmp/tmp_f1wu6t1.md:6: POSITIONING: airflow alternative: ａｉｒｆｌｏｗ＿ａｌｔｅｒｎａｔｉｖｅ
/tmp/tmp_f1wu6t1.md:7: POSITIONING: temporal clone: ｔｅｍｐｏｒａｌ＿ｃｌｏｎｅ
/tmp/tmp_f1wu6t1.md:8: POSITIONING: temporal alternative: ｔｅｍｐｏｒａｌ＿ａｌｔｅｒｎａｔｉｖｅ
/tmp/tmp_f1wu6t1.md:9: POSITIONING: temporal clone: t​emporal clone
summary: active=11 allowlisted=0 disclaimered=0 files_scanned=1
exit=1
```

### 8. synthetic unclosed disclaimer block

Observed exit: `2`

```text
check-product-positioning: scan error: /tmp/tmpr18x1agt.md: unclosed position-disclaimer block opened at line 1
exit=2
```

## Residual risk

- The scanner still canonicalizes phrases by simple separator folding; any new bypass shape needs a matching test.
- `allowlisted=0` remains in the summary for compatibility, but the historical allowlist marker is not used.
