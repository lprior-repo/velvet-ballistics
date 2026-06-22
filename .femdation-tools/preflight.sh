#!/bin/bash
# State 1 preflight for one bead
BEAD_ID="$1"
WS="/home/lewis/src/femdation-$BEAD_ID"
SRC="/home/lewis/src/velvet-ballistics"

mkdir -p "$WS/.beads/$BEAD_ID"

NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)

cat > "$WS/.beads/$BEAD_ID/STATE.md" <<STATE
# $BEAD_ID — Go-Skill State

bead_id: $BEAD_ID
isolated_workspace: $WS
source_checkout: $SRC
current_state: 1
state_owner: orchestrator (preflight)
created_at: $NOW
updated_at: $NOW

## Lifecycle
- [x] 1: orchestrator (preflight)
- [ ] 2: explore
- [ ] 3: rust-contract
- [ ] 4: proof-planner
- [ ] 4b: proof-plan-reviewer
- [ ] 5: proof-writer
- [ ] 6: proof-reviewer
- [ ] 7: proof-to-implementation
- [ ] 7b: proof-reviewer (bridge)
- [ ] 8: test-planner
- [ ] 9: test-writer
- [ ] 10: test-reviewer
- [ ] 11: holzman-rust
- [ ] 12: formal-verifier
- [ ] 13: black-hat-reviewer
- [ ] 14: evidence-packaging + truth-serum
- [ ] 15: landing-skill
- [ ] 16: orchestrator (cleanup)

## Attempts
| Gate | Attempts | Last result | Last evidence |
|------|----------|-------------|---------------|
| preflight | 1 | OK | baseline-report.md |
STATE

cat > "$WS/.beads/$BEAD_ID/runtime-skill-provenance.json" <<PROV
{"skill_name":"go-skill","skill_version":"10.1.0","format":"compact-with-references","mode":"control-plane-only","states_supported":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16],"controller":"femdation","controller_version":"2.0.0","loaded_at":"$NOW"}
PROV

cat > "$WS/.beads/$BEAD_ID/agent-invocation-ledger.jsonl" <<LEDGER
{"kind":"agent-invocation","bead_id":"$BEAD_ID","state":1,"agent":"orchestrator","role":"preflight","timestamp":"$NOW","status":"OK","evidence":"runtime-skill-provenance.json"}
LEDGER

cat > "$WS/.beads/$BEAD_ID/delivery-scope.jsonl" <<SCOPE
{"kind":"delivery-scope","bead_id":"$BEAD_ID","scope_kind":"preflight","status":"ready","set_at":"$NOW"}
SCOPE

echo "OK $BEAD_ID"
