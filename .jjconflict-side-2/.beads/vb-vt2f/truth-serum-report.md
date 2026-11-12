# Truth Serum Report — vb-vt2f State 13 owner-authorized substitute audit

STATUS: APPROVED

## Startup Controls Read

- Read `/home/lewis/.claude/skills/truth-serum/SKILL.md`. Applicable controls: direct execution required (`lines 25-40`), execution evidence must include command/stdout/exit status (`lines 34-40`, `132-133`), and subagent/delegated output cannot be laundered as proof (`lines 30-40`).
- Read `/home/lewis/.agents/skills/truth-serum/SKILL.md`. Relevant content matches the Claude copy; per startup instruction, this agents copy wins on conflict.

## Scope

- Bead audited: `vb-vt2f` only.
- State audited: State 13 truth-serum audit owner-authorized substitute packaging sublane only.
- Isolated workdir used for shell evidence: `/home/lewis/src/bd-vb-vt2f-bdd`.
- Modified artifact: `.beads/vb-vt2f/truth-serum-report.md` only.
- Production/test/proof artifacts were not modified.

## Execution Evidence

### Command — status, JSONL, and provenance assertions

Executed in `/home/lewis/src/bd-vb-vt2f-bdd`:

```text
python - <<'PY'
from pathlib import Path
import json, re, sys
root=Path.cwd()
base=root/'.beads'/'vb-vt2f'
print(f"PWD={root}")
assert str(root) == '/home/lewis/src/bd-vb-vt2f-bdd'
required_status = {
 'state13-provenance-waiver.md':'STATUS: APPROVED',
 'assurance-bundle.md':'STATUS: APPROVED',
 'final-evidence-decision.md':'STATUS: APPROVED',
 'formal-verification-report.md':'STATUS: APPROVED',
 'black-hat-review.md':'STATUS: APPROVED',
}
for name,want in required_status.items():
    lines=(base/name).read_text().splitlines()
    got=[(i+1,l) for i,l in enumerate(lines) if l.startswith('STATUS:')]
    print(f"{name} status={got}")
    assert got == [(3,want)]
rows=[]
for idx,line in enumerate((base/'verification-ledger.jsonl').read_text().splitlines(),1):
    obj=json.loads(line)
    obj['_line']=idx
    rows.append(obj)
counts={}
for obj in rows:
    counts[obj['result']]=counts.get(obj['result'],0)+1
bad=[(o['_line'],o['id'],o['result']) for o in rows if o['result'] in {'FAIL','FAIL_LOCAL','FAIL_REGRESSION','DEFERRED_GLOBAL'}]
print(f"ledger_rows={len(rows)} counts={counts} bad={bad}")
assert len(rows)==40
assert counts == {'PASS':35,'WAIVED':5}
assert bad == []
texts={name:(base/name).read_text() for name in ['state13-provenance-waiver.md','assurance-bundle.md','final-evidence-decision.md']}
combined='\n'.join(texts.values())
required_phrases=[
 'owner-authorized substitute',
 'not by the missing `evidence-packaging` specialist',
 'does not launder',
 'does not claim that the missing `evidence-packaging` agent ran',
 'not by claiming the missing `evidence-packaging` specialist ran',
 'must not be represented as output from the unavailable `evidence-packaging` specialist',
]
missing=[p for p in required_phrases if p not in combined]
print(f"provenance_required_phrases_missing={missing}")
assert missing == []
affirmative=[]
for name,text in texts.items():
    for i,line in enumerate(text.splitlines(),1):
        low=line.lower()
        mentions_run=('evidence-packaging' in low and any(v in low for v in [' ran',' executed',' completed',' produced'])) or ('specialist ran' in low) or ('agent ran' in low)
        if mentions_run and not any(neg in low for neg in ['does not claim','not by claiming','not by the missing','must not be represented']):
            affirmative.append((name,i,line))
print(f"affirmative_specialist_execution_claims={affirmative}")
assert affirmative == []
print('STATE13_TRUTH_SERUM_AUDIT=APPROVED')
PY
rc=$?; printf 'EXIT=%s\n' "$rc"; exit "$rc"
```

Observed stdout/stderr:

```text
PWD=/home/lewis/src/bd-vb-vt2f-bdd
state13-provenance-waiver.md status=[(3, 'STATUS: APPROVED')]
assurance-bundle.md status=[(3, 'STATUS: APPROVED')]
final-evidence-decision.md status=[(3, 'STATUS: APPROVED')]
formal-verification-report.md status=[(3, 'STATUS: APPROVED')]
black-hat-review.md status=[(3, 'STATUS: APPROVED')]
ledger_rows=40 counts={'PASS': 35, 'WAIVED': 5} bad=[]
provenance_required_phrases_missing=[]
affirmative_specialist_execution_claims=[]
STATE13_TRUTH_SERUM_AUDIT=APPROVED
EXIT=0
```

Exit status: `0`.

## Findings

- `state13-provenance-waiver.md`, `assurance-bundle.md`, and `final-evidence-decision.md` all disclose owner-authorized substitute provenance.
- They do not claim unavailable `evidence-packaging` specialist execution.
- The previous fallback provenance issue is resolved by owner-authorized substitute disclosure, not by pretending the evidence-packaging agent ran.
- `verification-ledger.jsonl` parses as 40 rows: `PASS: 35`, `WAIVED: 5`, no `FAIL`, `FAIL_LOCAL`, `FAIL_REGRESSION`, or `DEFERRED_GLOBAL` rows.
- Required upstream State 11/12 status artifacts are approved: `formal-verification-report.md` and `black-hat-review.md` both have exact `STATUS: APPROVED` lines.

## Decision

State 13 evidence packaging is adequate under the explicit owner-authorized substitute lane. The provenance is transparent and bounded; it is not specialist-provenance laundering.

APPROVED for femdation continuation.
