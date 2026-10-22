#!/usr/bin/env python3
"""
Independent verification of test-integrity checker justification hash binding.
Tests three scenarios:
1. Unrelated new_path should be rejected (bypass #1)
2. ABSENT new_path with existing file should be rejected (bypass #2)
3. Correct same-path justification should pass (control)
"""
import subprocess
import os
import tempfile
import shutil

CHECKER = "/tmp/vb-integrity-check"
REAL_GIT = "/usr/bin/git"

def git(root, *args):
    result = subprocess.run([REAL_GIT, "-C", root] + list(args), capture_output=True, text=True)
    if result.returncode != 0:
        print(f"git error: {result.stderr.strip()}")
    return result

def setup_base_test(root):
    """Base: strong assertion test + unrelated file."""
    os.makedirs(os.path.join(root, "tests"))
    with open(os.path.join(root, "tests", "behavior.rs"), "w") as f:
        f.write('#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n')
    with open(os.path.join(root, "README.md"), "w") as f:
        f.write("# Test Repo\n")
    git(root, "add", "tests/behavior.rs", "README.md")
    git(root, "commit", "-m", "initial commit")

def weaken_test(root):
    """Weaken the assertion in the same file."""
    with open(os.path.join(root, "tests", "behavior.rs"), "w") as f:
        f.write('#[test]\nfn behavior_test() {\n    assert!(2 + 2 == 4);\n}\n')
    git(root, "add", "tests/behavior.rs")
    git(root, "commit", "-m", "weaken assertion")

def test_scenario(name, root, setup_fn, justification_fn, expect_pass):
    """Set up a temp repo, run checker, test justification."""
    tmpdir = tempfile.mkdtemp()
    try:
        git(tmpdir, "init", "-q")
        git(tmpdir, "config", "user.email", "test@test.com")
        git(tmpdir, "config", "user.name", "Test")
        setup_fn(tmpdir)
        base_commit = git(tmpdir, "rev-parse", "HEAD").stdout.strip()
        base_blob = git(tmpdir, "rev-parse", f"{base_commit}:tests/behavior.rs").stdout.strip()
        
        weaken_test(tmpdir)
        current_commit = git(tmpdir, "rev-parse", "HEAD").stdout.strip()
        current_blob = git(tmpdir, "rev-parse", f"{current_commit}:tests/behavior.rs").stdout.strip()
        
        # Run checker to get findings
        result = subprocess.run([CHECKER, "--base", base_commit, "--target", current_commit],
                              capture_output=True, text=True, cwd=tmpdir)
        findings = [line for line in result.stdout.split("\n") if "WeakenedAssertion" in line]
        if not findings:
            print(f"  {name}: ERROR - no findings")
            return False
        
        finding_detail = findings[0].split("|")[2]
        
        # Run checker with justification
        result2 = subprocess.run([CHECKER, "--base", base_commit, "--target", current_commit,
                                 "--justifications", "justifications.tsv"],
                               capture_output=True, text=True, cwd=tmpdir)
        passed = (result2.returncode == 0)
        status = "PASS" if passed else "FAIL"
        expected_status = "PASS" if expect_pass else "FAIL"
        match = passed == expect_pass
        print(f"  {name}: {status} (expected {expected_status}) - {'OK' if match else 'MISMATCH'}")
        return match
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)

def scenario_unrelated_new_path(root):
    """Bypass #1: new_path points to unrelated file."""
    pass  # setup is done in test_scenario

def justification_unrelated(root, base_commit, finding_detail, path):
    """Justification record with new_path=README.md."""
    current_unrelated = git(root, "rev-parse", "HEAD:README.md").stdout.strip()
    with open(os.path.join(root, "justifications.tsv"), "w") as f:
        f.write(f"{base_commit}\ttests/behavior.rs\t{base_blob}\tREADME.md\t{current_unrelated}\tWeakenedAssertion\t{finding_detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")

def scenario_absent_new_path(root):
    """Edge case: new_path=ABSENT with file still existing."""
    pass

def justification_absent(root, base_commit, finding_detail, path):
    """Justification record with new_path=ABSENT."""
    current_unrelated = git(root, "rev-parse", "HEAD:README.md").stdout.strip()
    with open(os.path.join(root, "justifications.tsv"), "w") as f:
        f.write(f"{base_commit}\ttests/behavior.rs\t{base_blob}\tABSENT\tABSENT\tWeakenedAssertion\t{finding_detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")

def scenario_same_path_correct(root):
    """Control: correct justification with new_path=old_path."""
    pass

def justification_correct(root, base_commit, finding_detail, path):
    """Correct justification with new_path=old_path."""
    current_blob = git(root, "rev-parse", "HEAD:tests/behavior.rs").stdout.strip()
    current_unrelated = git(root, "rev-parse", "HEAD:README.md").stdout.strip()
    with open(os.path.join(root, "justifications.tsv"), "w") as f:
        f.write(f"{base_commit}\ttests/behavior.rs\t{base_blob}\ttests/behavior.rs\t{current_blob}\tWeakenedAssertion\t{finding_detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")

if __name__ == "__main__":
    print("Test-Integrity Checker Justification Hash Binding Verification")
    print("=" * 70)
    
    all_pass = True
    
    # Bypass #1: Unrelated new_path
    print("\n[1] Unrelated new_path (should be rejected):")
    def setup_and_justify_unrelated(root):
        setup_base_test(root)
        base = git(root, "rev-parse", "HEAD").stdout.strip()
        base_blob = git(root, "rev-parse", f"{base}:tests/behavior.rs").stdout.strip()
        weaken_test(root)
        current = git(root, "rev-parse", "HEAD").stdout.strip()
        result = subprocess.run([CHECKER, "--base", base, "--target", current],
                              capture_output=True, text=True, cwd=root)
        findings = [line for line in result.stdout.split("\n") if "WeakenedAssertion" in line]
        if findings:
            detail = findings[0].split("|")[2]
            current_unrelated = git(root, "rev-parse", f"{current}:README.md").stdout.strip()
            with open(os.path.join(root, "justifications.tsv"), "w") as f:
                f.write(f"{base}\ttests/behavior.rs\t{base_blob}\tREADME.md\t{current_unrelated}\tWeakenedAssertion\t{detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")
    tmpdir = tempfile.mkdtemp()
    try:
        git(tmpdir, "init", "-q")
        git(tmpdir, "config", "user.email", "test@test.com")
        git(tmpdir, "config", "user.name", "Test")
        setup_and_justify_unrelated(tmpdir)
        result = subprocess.run([CHECKER, "--base", git(tmpdir, "rev-parse", "HEAD~1").stdout.strip(),
                               "--target", git(tmpdir, "rev-parse", "HEAD").stdout.strip(),
                               "--justifications", "justifications.tsv"],
                              capture_output=True, text=True, cwd=tmpdir)
        passed = (result.returncode == 0)
        status = "PASS" if passed else "FAIL"
        expected = "FAIL"  # Should be rejected
        match = passed == False
        print(f"  Result: {status} (expected {expected}) - {'OK' if match else 'MISMATCH'}")
        all_pass = all_pass and match
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
    
    # Bypass #2: ABSENT new_path
    print("\n[2] ABSENT new_path (should be rejected):")
    tmpdir = tempfile.mkdtemp()
    try:
        git(tmpdir, "init", "-q")
        git(tmpdir, "config", "user.email", "test@test.com")
        git(tmpdir, "config", "user.name", "Test")
        setup_base_test(tmpdir)
        base = git(tmpdir, "rev-parse", "HEAD").stdout.strip()
        base_blob = git(tmpdir, "rev-parse", f"{base}:tests/behavior.rs").stdout.strip()
        weaken_test(tmpdir)
        current = git(tmpdir, "rev-parse", "HEAD").stdout.strip()
        result = subprocess.run([CHECKER, "--base", base, "--target", current],
                              capture_output=True, text=True, cwd=tmpdir)
        findings = [line for line in result.stdout.split("\n") if "WeakenedAssertion" in line]
        if findings:
            detail = findings[0].split("|")[2]
            current_unrelated = git(tmpdir, "rev-parse", f"{current}:README.md").stdout.strip()
            with open(os.path.join(tmpdir, "justifications.tsv"), "w") as f:
                f.write(f"{base}\ttests/behavior.rs\t{base_blob}\tABSENT\tABSENT\tWeakenedAssertion\t{detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")
        result = subprocess.run([CHECKER, "--base", base, "--target", current,
                               "--justifications", "justifications.tsv"],
                              capture_output=True, text=True, cwd=tmpdir)
        passed = (result.returncode == 0)
        status = "PASS" if passed else "FAIL"
        expected = "FAIL"  # Should be rejected
        match = passed == False
        print(f"  Result: {status} (expected {expected}) - {'OK' if match else 'MISMATCH'}")
        all_pass = all_pass and match
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
    
    # Control: Correct justification
    print("\n[3] Correct same-path justification (should pass):")
    tmpdir = tempfile.mkdtemp()
    try:
        git(tmpdir, "init", "-q")
        git(tmpdir, "config", "user.email", "test@test.com")
        git(tmpdir, "config", "user.name", "Test")
        setup_base_test(tmpdir)
        base = git(tmpdir, "rev-parse", "HEAD").stdout.strip()
        base_blob = git(tmpdir, "rev-parse", f"{base}:tests/behavior.rs").stdout.strip()
        weaken_test(tmpdir)
        current = git(tmpdir, "rev-parse", "HEAD").stdout.strip()
        current_blob = git(tmpdir, "rev-parse", f"{current}:tests/behavior.rs").stdout.strip()
        result = subprocess.run([CHECKER, "--base", base, "--target", current],
                              capture_output=True, text=True, cwd=tmpdir)
        findings = [line for line in result.stdout.split("\n") if "WeakenedAssertion" in line]
        if findings:
            detail = findings[0].split("|")[2]
            current_unrelated = git(tmpdir, "rev-parse", f"{current}:README.md").stdout.strip()
            with open(os.path.join(tmpdir, "justifications.tsv"), "w") as f:
                f.write(f"{base}\ttests/behavior.rs\t{base_blob}\ttests/behavior.rs\t{current_blob}\tWeakenedAssertion\t{detail}\t\tREADME.md\t{current_unrelated}\tvb-bxzxu\n")
        result = subprocess.run([CHECKER, "--base", base, "--target", current,
                               "--justifications", "justifications.tsv"],
                              capture_output=True, text=True, cwd=tmpdir)
        passed = (result.returncode == 0)
        status = "PASS" if passed else "FAIL"
        expected = "PASS"  # Should pass
        match = passed == True
        print(f"  Result: {status} (expected {expected}) - {'OK' if match else 'MISMATCH'}")
        all_pass = all_pass and match
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
    
    print("\n" + "=" * 70)
    if all_pass:
        print("ALL PROBE SCENARIOS VERIFIED: Hash binding is effective")
        print("Bypass #1 (unrelated path): REJECTED ✓")
        print("Bypass #2 (ABSENT path): REJECTED ✓")
        print("Control (correct path): ACCEPTED ✓")
    else:
        print("SOME PROBE SCENARIOS FAILED")
    
    exit(0 if all_pass else 1)
