# vb-kyyf State 13 Truth Serum Report

STATUS: APPROVED

## Startup Doctrine Cited

- `/home/lewis/.claude/skills/truth-serum/SKILL.md:8-10` requires direct execution evidence, no delegated proof, and command/evidence ownership.
- `/home/lewis/.agents/skills/truth-serum/SKILL.md:8-10` contains the same active rules and wins on conflict; no conflict observed.

## Scope

- Bead: `vb-kyyf` only.
- State: 13 truth-serum audit only.
- Attempt: `owner-authorized-substitute-1`.
- Isolated workspace: `/home/lewis/src/bd-vb-kyyf-bdd`.
- Output artifact: `.beads/vb-kyyf/truth-serum-report.md`.

## Execution Evidence

### Artifact/status/provenance/ledger validation

Command executed from `/home/lewis/src/bd-vb-kyyf-bdd`:

```text
python - <<'PY'
from pathlib import Path
import json
root = Path('/home/lewis/src/bd-vb-kyyf-bdd')
files = {
 'waiver': root/'.beads/vb-kyyf/state13-provenance-waiver.md',
 'bundle': root/'.beads/vb-kyyf/assurance-bundle.md',
 'decision': root/'.beads/vb-kyyf/final-evidence-decision.md',
 'formal': root/'.beads/vb-kyyf/formal-verification-report.md',
 'ledger': root/'.beads/vb-kyyf/verification-ledger.jsonl',
 'blackhat': root/'.beads/vb-kyyf/black-hat-review.md',
 'storage': root/'.evidence/vb-kyyf/storage-replay-resume.md',
}
for name, path in files.items():
    print(f'{name}: exists={path.exists()} size={path.stat().st_size if path.exists() else 0}')
    if not path.exists() or path.stat().st_size == 0:
        raise SystemExit(10)

for name in ['waiver','bundle','decision','formal','blackhat']:
    lines = files[name].read_text().splitlines()
    status = next((line for line in lines if line.startswith('STATUS: ')), None)
    print(f'{name}: {status}')
    if status != 'STATUS: APPROVED':
        raise SystemExit(11)

checks = [
 ('waiver substitute', 'owner-authorized substitute' in files['waiver'].read_text()),
 ('waiver non-agent', 'not represented as output from the missing `evidence-packaging` agent' in files['waiver'].read_text()),
 ('bundle substitute', 'owner-authorized substitute evidence packaging' in files['bundle'].read_text()),
 ('bundle non-agent', 'not output from a registered `evidence-packaging` OpenCode agent' in files['bundle'].read_text()),
 ('decision non-claim', 'does not claim execution by the missing `evidence-packaging` agent' in files['decision'].read_text()),
 ('formal state11 approved', 'STATUS: APPROVED' in files['formal'].read_text()),
 ('blackhat state12 approved', 'STATUS: APPROVED' in files['blackhat'].read_text() and 'APPROVED.' in files['blackhat'].read_text()),
 ('storage events4', 'events=4' in files['storage'].read_text() and 'seq=3: RunFinished' in files['storage'].read_text()),
]
for label, ok in checks:
    print(f'{label}: {ok}')
    if not ok:
        raise SystemExit(12)

ledger = []
with files['ledger'].open() as f:
    for idx, line in enumerate(f, 1):
        obj = json.loads(line)
        ledger.append(obj)
        print(f"ledger:{idx}: id={obj.get('id')} result={obj.get('result')} exit_status={obj.get('exit_status')} scope={obj.get('scope')}")
expected_ids = [f'PO-{i:03d}' for i in range(1, 11)]
if [o.get('id') for o in ledger] != expected_ids:
    raise SystemExit(13)
if any(o.get('result') != 'PASS' or o.get('exit_status') != 0 for o in ledger[:9]):
    raise SystemExit(14)
if ledger[9].get('result') != 'DEFERRED_GLOBAL' or ledger[9].get('exit_status') != 1:
    raise SystemExit(15)
print('VERDICT_INPUTS_OK')
PY
```

Observed stdout, exit 0:

```text
waiver: exists=True size=2068
bundle: exists=True size=12393
decision: exists=True size=1880
formal: exists=True size=4130
ledger: exists=True size=4976
blackhat: exists=True size=5712
storage: exists=True size=6232
waiver: STATUS: APPROVED
bundle: STATUS: APPROVED
decision: STATUS: APPROVED
formal: STATUS: APPROVED
blackhat: STATUS: APPROVED
waiver substitute: True
waiver non-agent: True
bundle substitute: True
bundle non-agent: True
decision non-claim: True
formal state11 approved: True
blackhat state12 approved: True
storage events4: True
ledger:1: id=PO-001 result=PASS exit_status=0 scope=bead-local
ledger:2: id=PO-002 result=PASS exit_status=0 scope=touched-crate
ledger:3: id=PO-003 result=PASS exit_status=0 scope=bead-local
ledger:4: id=PO-004 result=PASS exit_status=0 scope=touched-crate
ledger:5: id=PO-005 result=PASS exit_status=0 scope=touched-crate
ledger:6: id=PO-006 result=PASS exit_status=0 scope=bead-local
ledger:7: id=PO-007 result=PASS exit_status=0 scope=bead-local
ledger:8: id=PO-008 result=PASS exit_status=0 scope=protocol
ledger:9: id=PO-009 result=PASS exit_status=0 scope=bead-local
ledger:10: id=PO-010 result=DEFERRED_GLOBAL exit_status=1 scope=workspace
VERDICT_INPUTS_OK
```

### JSONL parse validation

Command executed from `/home/lewis/src/bd-vb-kyyf-bdd`:

```text
jq -c . .beads/vb-kyyf/verification-ledger.jsonl >/tmp/vb-kyyf-ledger-jq.out && jq -c . .beads/vb-kyyf/traceability-matrix.jsonl >/tmp/vb-kyyf-trace-jq.out && python - <<'PY'
from pathlib import Path
for path in [Path('/tmp/vb-kyyf-ledger-jq.out'), Path('/tmp/vb-kyyf-trace-jq.out')]:
    lines = path.read_text().splitlines()
    print(f'{path.name}: json_lines={len(lines)} first={lines[0][:80] if lines else ""}')
PY
```

Observed stdout, exit 0:

```text
vb-kyyf-ledger-jq.out: json_lines=10 first={"bead_id":"vb-kyyf","state":11,"sublane":"cap-unblock-canonical-aggregate-machi
vb-kyyf-trace-jq.out: json_lines=18 first={"contract_clause":"PRE-001","scenarios":["BDD-KYYF-001","BDD-KYYF-005","BDD-KYY
```

## Findings

- `state13-provenance-waiver.md`, `assurance-bundle.md`, and `final-evidence-decision.md` all carry `STATUS: APPROVED` and explicitly disclose owner-authorized substitute evidence packaging.
- The package does not launder provenance: waiver says it is not output from the missing `evidence-packaging` agent; assurance bundle says it is not output from a registered `evidence-packaging` OpenCode agent; final decision says it does not claim execution by the missing agent.
- State 11 and State 12 approval inputs are present: formal verification report and black-hat review both carry `STATUS: APPROVED`.
- `verification-ledger.jsonl` parses as JSONL, contains PO-001..PO-010 in order, has PO-001..PO-009 as `PASS` with `exit_status=0`, and has PO-010 as `DEFERRED_GLOBAL` with `exit_status=1`.
- Storage replay evidence contains non-stub durable replay facts (`events=4` and `seq=3: RunFinished`), matching the State 12 approval basis.

## Decision

APPROVED. The State 13 missing-agent issue is resolved by owner-authorized substitute disclosure, not by pretending an `evidence-packaging` agent ran. The audited artifacts adequately disclose substitute provenance, avoid unavailable-agent execution claims, and rest on approved State 11/12 evidence with valid JSONL ledger/trace inputs.
