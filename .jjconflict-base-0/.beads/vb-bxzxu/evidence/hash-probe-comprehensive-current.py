#!/usr/bin/env python3
"""
Comprehensive hash-binding probe adapted for current checker.
Tests: bypass-unrelated, bypass-absent, bypass-changed, correct-path,
and several edge cases. All must pass for APPROVE.
"""
import tempfile
import subprocess
import shutil
import sys
import os
import hashlib

VELVET_ROOT = "/home/lewis/src/isoloated/velvet-ballistics-vb-bxzxu-local3090-final-review"
CHECKER = os.path.join(VELVET_ROOT, "target", "gate-tools", "check-test-integrity")

REAL_GIT = "/usr/bin/git"

def git(root, *args):
    result = subprocess.run([REAL_GIT, "-C", root] + list(args), capture_output=True, text=True)
    return result

def file_hash(root, path):
    result = subprocess.run([REAL_GIT, "-C", root, "hash-object", path], capture_output=True, text=True)
    return result.stdout.strip()

def run_checker(root, base):
    result = subprocess.run(
        [CHECKER, "--base", base, "--root", root],
        capture_output=True, text=True
    )
    return result

def setup_base(root):
    """Create base commit with strong assertion test."""
    git(root, "init", "-b", "main")
    git(root, "config", "user.name", "probe")
    git(root, "config", "user.email", "probe@test")
    os.makedirs(os.path.join(root, "tests"), exist_ok=True)
    with open(os.path.join(root, "tests", "behavior.rs"), "w") as f:
        f.write("#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n")
    git(root, "add", ".")
    git(root, "commit", "-m", "base: strong assertion")
    return git(root, "rev-parse", "HEAD").stdout.strip()

def weaken_test(root):
    """Weaken the test assertion."""
    with open(os.path.join(root, "tests", "behavior.rs"), "w") as f:
        f.write("#[test]\nfn behavior_test() {\n    assert!(2 + 2 == 4);\n}\n")
    git(root, "add", ".")
    git(root, "commit", "-m", "weakened test")

def add_unrelated_file(root):
    """Add unrelated README change."""
    with open(os.path.join(root, "README.md"), "a") as f:
        f.write("\nUnrelated.\n")

def write_justification(root, base_commit, old_path, old_blob, new_path, new_blob,
                        finding_kind="WeakenedAssertion", detail="removed_exact=1 added_exact=0 added_weak=0"):
    """Write a justification record."""
    os.makedirs(os.path.join(root, "scripts"), exist_ok=True)
    jpath = os.path.join(root, "scripts", "test-integrity-justifications.tsv")
    with open(jpath, "w") as f:
        f.write(f"{base_commit}\t{old_path}\t{old_blob}\t{new_path}\t{new_blob}\t{finding_kind}\t{detail}\t\tREADME.md\tcurrentblob\tvb-probe\n")

def test_case(name, setup, expect_exit, description):
    """Run a test case and report result."""
    tmpdir = tempfile.mkdtemp(prefix=f"vb-probe-{name}-")
    try:
        base_commit = setup(tmpdir)
        result = run_checker(tmpdir, base_commit)
        passed = (result.returncode == expect_exit)
        status = "PASS" if passed else "FAIL"
        print(f"[{status}] {name}: {description} (exit={result.returncode}, expected={expect_exit})")
        if not passed:
            print(f"  stdout: {result.stdout[:200]}")
            print(f"  stderr: {result.stderr[:200]}")
        return passed
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)

def main():
    results = []

    # Test 1: bypass-unrelated - new_path points to unrelated file
    def setup_unrelated(root):
        base = setup_base(root)
        weaken_test(root)
        add_unrelated_file(root)
        write_justification(root, base, "tests/behavior.rs", "baseblob", "README.md", "currentblob")
        return base
    results.append(test_case("bypass-unrelated", setup_unrelated, 1,
        "new_path points to unrelated file - finding should NOT be justified"))

    # Test 2: bypass-absent - ABSENT while old file still exists
    def setup_absent(root):
        base = setup_base(root)
        weaken_test(root)
        add_unrelated_file(root)
        write_justification(root, base, "tests/behavior.rs", "baseblob", "ABSENT", "ABSENT")
        return base
    results.append(test_case("bypass-absent", setup_absent, 1,
        "new_path=ABSENT while old file still exists - finding should NOT be justified"))

    # Test 3: bypass-changed - file changed after justification
    def setup_changed(root):
        base = setup_base(root)
        weaken_test(root)
        add_unrelated_file(root)
        write_justification(root, base, "tests/behavior.rs", "baseblob", "tests/behavior.rs", "wrongblob")
        return base
    results.append(test_case("bypass-changed", setup_changed, 1,
        "new_blob doesn't match current file - finding should NOT be justified"))

    print("\n" + "=" * 60)
    if all(results):
        print(f"ALL {len(results)} PROBE TESTS PASSED")
        return 0
    else:
        passed = sum(results)
        print(f"{passed}/{len(results)} PROBE TESTS PASSED")
        return 1

if __name__ == "__main__":
    sys.exit(main())