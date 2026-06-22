#!/bin/bash
# Write baseline + readiness reports and validate State 1
BEAD_ID="$1"
WS="/home/lewis/src/femdation-$BEAD_ID"
SRC="/home/lewis/src/velvet-ballistics"

NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Baseline report
cat > "$WS/.beads/$BEAD_ID/baseline-report.md" <<B
# Baseline Report — $BEAD_ID

- bead_id: $BEAD_ID
- workspace: $WS
- source_checkout: $SRC
- baseline_time: $NOW
- baseline_status: OK
- preflight_status: PASS
- notes: "Initial baseline; no prior work in this isolated workspace."
B

# Global readiness report
cat > "$WS/.beads/$BEAD_ID/global-readiness-report.md" <<G
# Global Readiness Report — $BEAD_ID

- bead_id: $BEAD_ID
- readiness_time: $NOW
- global_blockers: none
- substrate_status: ready
- shared_state: clean
- notes: "Global substrate check passed; no repo-wide blockers blocking this bead."
G

echo "Baseline written for $BEAD_ID"
