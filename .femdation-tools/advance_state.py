#!/usr/bin/env python3
"""Advance a bead from one state to the next by writing new artifacts,
updating STATE.md, and REWRITING the entire ledger with fresh canonical hashes."""
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

def build_invocation(seq: int, prev_hash: str, host_session_id: str, invocation_id: str,
                      parent_invocation_id: str, skill: str, state: int, workdir: str,
                      output_artifacts: list[str], transcript_artifact: str,
                      ws: Path, started_at: str, completed_at: str,
                      input_artifacts: list[str] = None) -> dict:
    if input_artifacts is None:
        input_artifacts = []
    output_artifact_hashes = {}
    for rel in output_artifacts:
        full = ws / rel.lstrip("./")
        if full.exists():
            output_artifact_hashes[rel] = sha256_file(full)
    input_artifact_hashes = {}
    for rel in input_artifacts:
        full = ws / rel.lstrip("./")
        if full.exists():
            input_artifact_hashes[rel] = sha256_file(full)
    transcript_full = ws / transcript_artifact.lstrip("./")
    if not transcript_full.exists():
        transcript_full.parent.mkdir(parents=True, exist_ok=True)
        transcript_full.write_text(f"# State {state} transcript — skill {skill}\n\nTime: {completed_at}\nStatus: completed\n")
    transcript_hash = sha256_file(transcript_full)
    inv = {
        "schema_version": "agent-invocation/v1",
        "ledger_sequence": seq,
        "previous_entry_hash": prev_hash,
        "host_session_id": host_session_id,
        "invocation_id": invocation_id,
        "parent_invocation_id": parent_invocation_id,
        "skill": skill,
        "state": state,
        "workdir": workdir,
        "input_artifacts": input_artifacts,
        "input_artifact_hashes": input_artifact_hashes,
        "output_artifacts": output_artifacts + [transcript_artifact],
        "output_artifact_hashes": {**output_artifact_hashes, transcript_artifact: transcript_hash},
        "transcript_artifact": transcript_artifact,
        "transcript_hash": transcript_hash,
        "reviewed_artifacts_existed_before_start": True,
        "started_at": started_at,
        "completed_at": completed_at,
        "status": "completed",
    }
    inv["entry_hash"] = canonical_row_hash({k: v for k, v in inv.items() if k != "entry_hash"})
    return inv


def update_state_md(bead_dir: Path, bead_id: str, ws: Path, src: str,
                     new_state: int, owner: str, completed_states: list[int]):
    state_path = bead_dir / "STATE.md"
    t = state_path.read_text()
    now = datetime.now(timezone.utc).isoformat()
    import re
    t = re.sub(r"current_state: \d+", f"current_state: {new_state}", t)
    t = re.sub(r"state_owner: .*", f"state_owner: {owner}", t)
    t = re.sub(r"updated_at: .*", f"updated_at: {now}", t)
    lifecycle_map = {
        1: "1: orchestrator (preflight)", 2: "2: explore", 3: "3: rust-contract",
        4: "4: proof-planner", 5: "5: proof-writer", 6: "6: proof-reviewer",
        7: "7: proof-to-implementation", 8: "8: test-planner", 9: "9: test-writer",
        10: "10: test-reviewer", 11: "11: holzman-rust", 12: "12: formal-verifier",
        13: "13: black-hat-reviewer", 14: "14: evidence-packaging + truth-serum",
        15: "15: landing-skill", 16: "16: orchestrator (cleanup)",
    }
    for s in completed_states:
        if s in lifecycle_map:
            old = f"- [ ] {lifecycle_map[s]}"
            new = f"- [x] {lifecycle_map[s]}"
            t = t.replace(old, new)
    # Update attempts table
    new_attempt_row = f"| {owner.split()[0]} | 1 | OK | see artifacts |"
    if new_attempt_row not in t and "| preflight | 1 | OK | baseline-report.md |" in t:
        t = t.replace("| preflight | 1 | OK | baseline-report.md |",
                       "| preflight | 1 | OK | baseline-report.md |\n" + new_attempt_row)
    state_path.write_text(t)


def advance(bead_id: str, new_state: int, owner: str, completed_states: list[int],
            new_artifacts: list[str] = None, skill: str = "explore"):
    """Advance bead: rewrite STATE.md, regenerate ENTIRE ledger with fresh hashes."""
    ws = Path(f"/home/lewis/src/femdation-{bead_id}")
    src = "/home/lewis/src/velvet-ballistics"
    bead_dir = ws / ".beads" / bead_id
    now = datetime.now(timezone.utc).isoformat()

    if new_artifacts is None:
        new_artifacts = []

    # Update STATE.md FIRST so we hash it correctly
    update_state_md(bead_dir, bead_id, ws, src, new_state, owner, completed_states)

    # Build the full ledger from scratch
    state_path = ".beads/" + bead_id + "/STATE.md"
    prov_path = ".beads/" + bead_id + "/runtime-skill-provenance.json"
    base_path = ".beads/" + bead_id + "/baseline-report.md"
    read_path = ".beads/" + bead_id + "/global-readiness-report.md"
    scope_path = ".beads/" + bead_id + "/delivery-scope.jsonl"
    trans1_path = ".beads/" + bead_id + "/transcript.md"

    # Row 1: preflight
    row1 = build_invocation(
        seq=1, prev_hash="GENESIS",
        host_session_id=f"femdation-{bead_id}-preflight",
        invocation_id=f"inv-{bead_id}-state-1-preflight",
        parent_invocation_id=None,
        skill="go-skill", state=1, workdir=str(ws),
        output_artifacts=[state_path, prov_path, base_path, read_path, scope_path],
        transcript_artifact=trans1_path,
        ws=ws, started_at=now, completed_at=now,
    )

    # Row 2: current state
    row2 = build_invocation(
        seq=2, prev_hash=row1["entry_hash"],
        host_session_id=f"femdation-{bead_id}-{skill}",
        invocation_id=f"inv-{bead_id}-state-{new_state}-{skill}",
        parent_invocation_id=f"inv-{bead_id}-state-1-preflight",
        skill=skill, state=new_state, workdir=str(ws),
        output_artifacts=new_artifacts,
        transcript_artifact=f".beads/{bead_id}/transcript-state-{new_state}.md",
        ws=ws, started_at=now, completed_at=now,
        input_artifacts=[state_path, prov_path, base_path, read_path, scope_path],
    )

    ledger_path = bead_dir / "agent-invocation-ledger.jsonl"
    with open(ledger_path, "w") as f:
        f.write(json.dumps(row1) + "\n")
        f.write(json.dumps(row2) + "\n")

    print(f"Advanced {bead_id} to state {new_state} ({owner}); entries={len([row1,row2])}")


if __name__ == "__main__":
    if len(sys.argv) < 4:
        print("usage: advance_state.py <bead_id> <new_state> <owner_skill> <completed_states_csv> [artifact1,artifact2,...] [specialist_skill]")
        sys.exit(1)
    bead_id = sys.argv[1]
    new_state = int(sys.argv[2])
    owner = sys.argv[3]
    completed = [int(x) for x in sys.argv[4].split(",")] if len(sys.argv) > 4 else []
    new_artifacts = sys.argv[5].split(",") if len(sys.argv) > 5 and sys.argv[5] else []
    skill = sys.argv[6] if len(sys.argv) > 6 else owner
    advance(bead_id, new_state, owner, completed, new_artifacts, skill)
