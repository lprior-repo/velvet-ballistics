#!/usr/bin/env python3
"""
Probes the test-integrity checker's justification mechanism for edge cases:
1. new_path points to unrelated file (bypass #1 - already confirmed)
2. new_path=ABSENT with actual file existing (should be rejected correctly)
3. Stale record after actual test changes
"""
import subprocess
import tempfile
import shutil
import os

VELVET_ROOT = "/home/lewis/src/isoloated/velvet-ballistics-vb-bxzxu-local5090-hash-review"
CHECKER = os.path.join(VELVET_ROOT, "target", "gate-tools", "check-test-integrity")
REAL_GIT = "/usr/bin/git"

def git(root, *args):
    result = subprocess.run([REAL_GIT, "-C", root] + list(args), capture_output=True, text=True)
    if result.returncode != 0:
        print(f"git {args} failed: {result.stderr}", file=__import__('sys').stderr)
    return result

def file_hash(root, path):
    result = subprocess.run([REAL_GIT, "-C", root, "hash-object", path], capture_output=True, text=True)
    return result.stdout.strip()

def run_checker(root, base):
    env = os.environ.copy()
    env["TEST_INTEGRITY_BASE"] = base
    result = subprocess.run([CHECKER, "--base", base], cwd=root, capture_output=True, text=True, env=env)
    return result

def test_scenario(name, setup_fn, justification_fn=None, expect_pass=False):
    """Set up a temp repo, run checker, test justification."""
    print(f"\n{'='*60}")
    print(f"SCENARIO: {name}")
    print('='*60)
    
    tmpdir = tempfile.mkdtemp(prefix="vb-hash-probe-")
    root = tmpdir
    
    try:
        # Initialize repo
        git(root, "init", "-b", "main")
        git(root, "config", "user.email", "test@example.com")
        git(root, "config", "user.name", "Test User")
        
        # Run setup
        base_commit, finding_detail = setup_fn(root)
        
        # Run checker without justification
        result = run_checker(root, base_commit)
        print(f"Without justification: exit={result.returncode}")
        print(f"  stderr: {result.stderr.strip()}")
        
        if not finding_detail:
            # Try to extract from output
            for line in result.stderr.split('\n'):
                if "WeakenedAssertion|" in line or "DeletedTestFile|" in line:
                    parts = line.split("|")
                    if len(parts) >= 3:
                        finding_detail = parts[2]
                        break
        
        if justification_fn:
            # Write justification
            justification_path = os.path.join(root, "scripts", "test-integrity-justifications.tsv")
            os.makedirs(os.path.join(root, "scripts"), exist_ok=True)
            justification_fn(root, base_commit, finding_detail, justification_path)
            
            # Run checker with justification
            result = run_checker(root, base_commit)
            print(f"With justification: exit={result.returncode}")
            print(f"  stderr: {result.stderr.strip()}")
            
            if expect_pass and result.returncode == 0:
                print("  RESULT: PASS (finding justified)")
            elif not expect_pass and result.returncode == 1:
                print("  RESULT: FAIL (finding un-justified)")
            elif expect_pass and result.returncode != 0:
                print(f"  RESULT: UNEXPECTED FAIL (exit={result.returncode})")
            else:
                print(f"  RESULT: UNEXPECTED PASS")
        else:
            if result.returncode == 1:
                print("  RESULT: FAIL (expected)")
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)


def setup_base_test(root):
    """Base: strong assertion test + unrelated file."""
    test_dir = os.path.join(root, "tests")
    os.makedirs(test_dir, exist_ok=True)
    
    with open(os.path.join(test_dir, "behavior.rs"), "w") as f:
        f.write('#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n')
    
    with open(os.path.join(root, "README.md"), "w") as f:
        f.write("# Unrelated\n")
    
    git(root, "add", "-A")
    git(root, "commit", "-m", "base: strong assertion test")
    return None, None

def setup_weakened_test(root):
    """Weaken the assertion in the same file."""
    with open(os.path.join(root, "tests", "behavior.rs"), "w") as f:
        f.write('#[test]\nfn behavior_test() {\n    assert!(2 + 2 == 4);\n}\n')

def scenario_unrelated_new_path(root):
    """Bypass #1: new_path points to unrelated file."""
    base_commit, _ = setup_base_test(root)
    setup_weakened_test(root)
    base_commit = git(root, "rev-parse", "HEAD").stdout.strip()
    return base_commit, "removed_exact=1 added_exact=0 added_weak=0"

def justification_unrelated(root, base_commit, finding_detail, path):
    """Justification record with new_path=README.md."""
    # Get base blob from the base commit
    result = subprocess.run([REAL_GIT, "-C", root, "show", f"{base_commit}:tests/behavior.rs"], capture_output=True)
    base_blob = subprocess.run(["git", "hash-object", "--stdin"], input=result.stdout, capture_output=True).stdout.strip().decode()
    
    current_unrelated = file_hash(root, "README.md")
    
    with open(path, "w") as f:
        f.write(f"{base_commit}\ttests/behavior.rs\t{base_blob}\tREADME.md\t{current_unrelated}\tWeakenedAssertion\t{finding_detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")

def scenario_absent_new_path(root):
    """Edge case: new_path=ABSENT with file still existing."""
    base_commit, _ = setup_base_test(root)
    setup_weakened_test(root)
    return git(root, "rev-parse", "HEAD").stdout.strip(), "removed_exact=1 added_exact=0 added_weak=0"

def justification_absent(root, base_commit, finding_detail, path):
    """Justification record with new_path=ABSENT."""
    # Need base blob
    result = subprocess.run([REAL_GIT, "-C", root, "show", f"{base_commit}:tests/behavior.rs"], capture_output=True)
    base_blob = subprocess.run(["git", "hash-object", "--stdin"], input=result.stdout, capture_output=True).stdout.strip().decode()
    
    # Use README as review artifact
    current_unrelated = file_hash(root, "README.md")
    
    with open(path, "w") as f:
        f.write(f"{base_commit}\ttests/behavior.rs\t{base_blob}\tABSENT\tABSENT\tWeakenedAssertion\t{finding_detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")

def scenario_same_path_correct(root):
    """Control: correct justification with new_path=old_path."""
    base_commit, _ = setup_base_test(root)
    setup_weakened_test(root)
    return git(root, "rev-parse", "HEAD").stdout.strip(), "removed_exact=1 added_exact=0 added_weak=0"

def justification_correct(root, base_commit, finding_detail, path):
    """Correct justification with new_path=old_path."""
    # Get base blob
    result = subprocess.run([REAL_GIT, "-C", root, "show", f"{base_commit}:tests/behavior.rs"], capture_output=True)
    base_blob = subprocess.run(["git", "hash-object", "--stdin"], input=result.stdout, capture_output=True).stdout.strip().decode()
    
    # Get current blob of the actual test
    current_blob = file_hash(root, "tests/behavior.rs")
    current_unrelated = file_hash(root, "README.md")
    
    with open(path, "w") as f:
        f.write(f"{base_commit}\ttests/behavior.rs\t{base_blob}\ttests/behavior.rs\t{current_blob}\tWeakenedAssertion\t{finding_detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")

if __name__ == "__main__":
    print("Test-Integrity Checker Justification Bypass Probes")
    print("="*60)
    
    # Probe 1: Bypass with unrelated new_path
    test_scenario(
        "Bypass: new_path points to unrelated file",
        scenario_unrelated_new_path,
        justification_unrelated,
        expect_pass=True
    )
    
    # Probe 2: Edge case with new_path=ABSENT
    test_scenario(
        "Edge: new_path=ABSENT with file existing",
        scenario_absent_new_path,
        justification_absent,
        expect_pass=False
    )
    
    # Probe 3: Control with correct new_path=old_path
    test_scenario(
        "Control: correct new_path=old_path",
        scenario_same_path_correct,
        justification_correct,
        expect_pass=True
    )
    
    print("\n" + "="*60)
    print("SUMMARY:")
    print("  Bypass #1 (unrelated new_path): CONFIRMED")
    print("  new_path=ABSENT edge case: needs manual verification")
    print("  Control (correct path): should pass")
