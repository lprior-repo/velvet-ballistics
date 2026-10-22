#!/usr/bin/env python3
"""
Reproduces the new_path bypass in the test-integrity justification checker.

Scenario:
1. Base commit: test file with strong assertion (assert_eq)
2. Current working tree: same test file with weakened assertion (is_some)
3. Also create an unrelated file that is unchanged
4. Create a justification record that binds the finding but points new_path
   to the unrelated file, not the actual weakened test location
5. The checker should reject the justification (finding stays un-justified)
   but the draft accepts it, demonstrating the bypass.

Expected (correct behavior): Finding un-justified, checker exits 1
Actual (draft behavior): Finding justified, checker exits 0 (bypass)
"""

import tempfile
import subprocess
import shutil
import sys
import os

VELVET_ROOT = "/home/lewis/src/isoloated/velvet-ballistics-vb-bxzxu-local5090-hash-review"
REAL_GIT = "/usr/bin/git"
CHECKER = os.path.join(VELVET_ROOT, "target", "gate-tools", "check-test-integrity")

def git(root, *args):
    """Run git command in root directory."""
    result = subprocess.run(
        [REAL_GIT, "-C", root] + list(args),
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"git {args} failed: {result.stderr}", file=sys.stderr)
    return result

def file_hash(root, path):
    result = subprocess.run(
        [REAL_GIT, "-C", root, "hash-object", path],
        capture_output=True, text=True
    )
    return result.stdout.strip()

def main():
    tmpdir = tempfile.mkdtemp(prefix="vb-hash-probe-")
    root = tmpdir
    
    try:
        # Initialize git repo
        git(root, "init", "-b", "main")
        git(root, "config", "user.email", "test@example.com")
        git(root, "config", "user.name", "Test User")
        
        # Create test directory
        test_dir = os.path.join(root, "tests")
        os.makedirs(test_dir, exist_ok=True)
        
        # Base commit: test file with strong assertion
        strong_test = '#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n'
        test_path = os.path.join(test_dir, "behavior.rs")
        with open(test_path, "w") as f:
            f.write(strong_test)
        
        # Also create an unrelated file (unchanged throughout)
        unrelated_path = os.path.join(root, "README.md")
        with open(unrelated_path, "w") as f:
            f.write("# Unrelated\n")
        
        git(root, "add", "-A")
        git(root, "commit", "-m", "base: strong assertion test")
        
        base_commit = git(root, "rev-parse", "HEAD").stdout.strip()
        print(f"Base commit: {base_commit}")
        
        # Get blob hashes at base
        base_test_blob = file_hash(root, "tests/behavior.rs")
        base_unrelated_blob = file_hash(root, "README.md")
        print(f"Base test blob: {base_test_blob}")
        print(f"Base unrelated blob: {base_unrelated_blob}")
        
        # Current working tree: weaken the assertion in the SAME file
        weak_test = '#[test]\nfn behavior_test() {\n    assert!(2 + 2 == 4);\n}\n'
        with open(test_path, "w") as f:
            f.write(weak_test)
        
        # Get blobs before running checker
        current_test_blob = file_hash(root, "tests/behavior.rs")
        current_unrelated_blob = file_hash(root, "README.md")
        print(f"Current test blob: {current_test_blob}")
        print(f"Current unrelated blob: {current_unrelated_blob}")
        
        # Run checker without justifications to get the exact finding detail
        env = os.environ.copy()
        env["TEST_INTEGRITY_BASE"] = base_commit
        
        print(f"Checker: {CHECKER}")
        print(f"CWD: {root}")
        print(f"Base: {base_commit}")
        print(f"TEST_INTEGRITY_BASE: {env.get('TEST_INTEGRITY_BASE', 'NOT SET')}")
        result = subprocess.run(
            [CHECKER, "--base", base_commit],
            cwd=root,
            capture_output=True, text=True,
            env=env
        )
        print(f"Result: stdout={repr(result.stdout)}, stderr={repr(result.stderr)}, returncode={result.returncode}")
        print(f"\n--- Checker output (no justifications) ---")
        print("stdout:", repr(result.stdout))
        print("stderr:", repr(result.stderr))
        if result.returncode != 1:
            print(f"WARNING: Expected exit 1 without justifications, got {result.returncode}")
        
        # Parse the finding detail from output
        finding_detail = None
        for line in result.stderr.split('\n'):
            if line.startswith("WeakenedAssertion|"):
                parts = line.split("|")
                if len(parts) >= 3:
                    finding_detail = parts[2]
                    print(f"\nExtracted finding detail: {finding_detail}")
        
        if not finding_detail:
            print("ERROR: Could not extract finding detail", file=sys.stderr)
            return 1
        
        # Create justification record that binds the finding but points new_path
        # to the unrelated file instead of the actual weakened test location
        justification_path = os.path.join(root, "scripts", "test-integrity-justifications.tsv")
        os.makedirs(os.path.join(root, "scripts"), exist_ok=True)
        
        # Write justification record (11 columns)
        # base_commit, old_path, old_blob, new_path, new_blob, finding_kind, finding_detail,
        # replacement_paths, review_artifact_path, review_artifact_blob, bead_id
        # BYPASS: new_path points to README.md (unrelated file), not the actual weakened test
        with open(justification_path, "w") as f:
            f.write(f"{base_commit}\ttests/behavior.rs\t{base_test_blob}\tREADME.md\t{current_unrelated_blob}\tWeakenedAssertion\t{finding_detail}\t\tREADME.md\t{current_unrelated_blob}\tvb-bxzxu\n")

        
        print(f"\nWrote justification record pointing new_path to unrelated file")
        print(f"old_path=tests/behavior.rs (correct)")
        print(f"new_path=README.md (unrelated file, bypass)")
        
        # Run checker again - it should reject this justification
        result = subprocess.run(
            [CHECKER, "--base", base_commit],
            cwd=root,
            capture_output=True, text=True,
            env=env
        )
        print(f"\n--- Checker output (with bypass justification) ---")
        print(result.stdout)
        print(f"Exit code: {result.returncode}")
        
        if result.returncode == 0:
            print("\nBUG CONFIRMED: Bypass succeeded - finding was justified despite")
            print("new_path pointing to an unrelated file. The actual weakened test")
            print("at tests/behavior.rs was never bound to its current blob.")
        elif result.returncode == 1:
            print("\nNo bypass: Finding correctly remained un-justified.")
        else:
            print(f"\nUnexpected exit code: {result.returncode}")
            if result.stderr:
                print(result.stderr)
        
        return result.returncode
        
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)

if __name__ == "__main__":
    sys.exit(main())
