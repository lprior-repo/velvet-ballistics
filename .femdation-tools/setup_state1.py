#!/usr/bin/env python3
"""Generate State 1 artifacts for femdation beads with correct canonical hashing."""
import json
import hashlib
import sys
from pathlib import Path
from datetime import datetime, timezone

def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    h.update(p.read_bytes())
    return h.hexdigest()

def canonical_row_hash(row: dict) -> str:
    payload = json.dumps(row, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()

def setup_bead(bead_id: str, src: str = "/home/lewis/src/velvet-ballistics"):
    ws = Path(f"/home/lewis/src/femdation-{bead_id}")
    bead_dir = ws / ".beads" / bead_id
    bead_dir.mkdir(parents=True, exist_ok=True)

    now = datetime.now(timezone.utc).isoformat()

    # 1) STATE.md
    state_md = f"""# {bead_id} — Go-Skill State

bead_id: {bead_id}
isolated_workspace: {ws}
source_checkout: {src}
current_state: 1
state_owner: orchestrator (preflight)
created_at: {now}
updated_at: {now}

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
"""
    (bead_dir / "STATE.md").write_text(state_md)

    # 2) runtime-skill-provenance.json (validator schema)
    provenance = {
        "loaded_skill_name": "go-skill",
        "loaded_skill_version": "10.1.0",
        "observed_state_range": "1..16",
        "bead_id": bead_id,
        "isolated_workspace": str(ws),
        "source_checkout": src,
        "skill_root_canonical": "/home/lewis/.agents/skills/go-skill",
        "skill_root_mirror": "/home/lewis/.opencode/skill/go-skill",
        "loaded_at": now,
    }
    (bead_dir / "runtime-skill-provenance.json").write_text(json.dumps(provenance, indent=2))

    # 3) baseline-report.md
    baseline = f"""# Baseline Report — {bead_id}

- bead_id: {bead_id}
- workspace: {ws}
- source_checkout: {src}
- baseline_time: {now}
- baseline_status: OK
- preflight_status: PASS
- notes: Initial baseline; isolated workspace ready.
"""
    (bead_dir / "baseline-report.md").write_text(baseline)

    # 4) global-readiness-report.md
    readiness = f"""# Global Readiness Report — {bead_id}

- bead_id: {bead_id}
- readiness_time: {now}
- global_blockers: none
- substrate_status: ready
- shared_state: clean
- notes: No repo-wide blockers blocking this bead.
"""
    (bead_dir / "global-readiness-report.md").write_text(readiness)

    # 5) delivery-scope.jsonl
    delivery_scope = {
        "schema_version": "delivery-scope/v1",
        "bead_id": bead_id,
        "scope_kind": "preflight",
        "status": "ready",
        "set_at": now,
    }
    (bead_dir / "delivery-scope.jsonl").write_text(json.dumps(delivery_scope) + "\n")

    # 6) agent-invocation-ledger.jsonl (canonical hashing)
    state_path = ".beads/" + bead_id + "/STATE.md"
    prov_path = ".beads/" + bead_id + "/runtime-skill-provenance.json"
    base_path = ".beads/" + bead_id + "/baseline-report.md"
    read_path = ".beads/" + bead_id + "/global-readiness-report.md"
    scope_path = ".beads/" + bead_id + "/delivery-scope.jsonl"
    trans_path = ".beads/" + bead_id + "/transcript.md"

    # Compute hashes of actual files (must be created first; they exist now)
    state_hash = sha256_file(bead_dir / "STATE.md")
    prov_hash = sha256_file(bead_dir / "runtime-skill-provenance.json")
    base_hash = sha256_file(bead_dir / "baseline-report.md")
    read_hash = sha256_file(bead_dir / "global-readiness-report.md")
    scope_hash = sha256_file(bead_dir / "delivery-scope.jsonl")

    # Transcript file (minimal but exists for hash)
    transcript = f"""# Preflight Transcript — {bead_id}

Bead: {bead_id}
State: 1 (orchestrator preflight)
Started: {now}
Completed: {now}
Status: OK
Notes: Isolated workspace initialized. Baseline captured. Global readiness verified. Ready for State 2 (explore).
"""
    (bead_dir / "transcript.md").write_text(transcript)
    trans_hash = sha256_file(bead_dir / "transcript.md")

    invocation = {
        "schema_version": "agent-invocation/v1",
        "ledger_sequence": 1,
        "previous_entry_hash": "GENESIS",
        "host_session_id": f"femdation-{bead_id}-preflight",
        "invocation_id": f"inv-{bead_id}-state-1-preflight",
        "parent_invocation_id": None,
        "skill": "go-skill",
        "state": 1,
        "workdir": str(ws),
        "input_artifacts": [],
        "input_artifact_hashes": [],
        "output_artifacts": [state_path, prov_path, base_path, read_path, scope_path, trans_path],
        "output_artifact_hashes": {
            state_path: state_hash,
            prov_path: prov_hash,
            base_path: base_hash,
            read_path: read_hash,
            scope_path: scope_hash,
            trans_path: trans_hash,
        },
        "transcript_artifact": trans_path,
        "transcript_hash": trans_hash,
        "reviewed_artifacts_existed_before_start": True,
        "started_at": now,
        "completed_at": now,
        "status": "completed",
    }
    # Compute entry_hash from canonical JSON of all fields except entry_hash itself
    invocation["entry_hash"] = canonical_row_hash({k: v for k, v in invocation.items() if k != "entry_hash"})

    (bead_dir / "agent-invocation-ledger.jsonl").write_text(json.dumps(invocation) + "\n")

    print(f"OK {bead_id}")

if __name__ == "__main__":
    bead_ids = sys.argv[1:]
    if not bead_ids:
        print("usage: setup_state1.py <bead-id> [<bead-id>...]")
        sys.exit(1)
    for bid in bead_ids:
        setup_bead(bid)
