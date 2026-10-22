#!/usr/bin/env python3
"""
Adapted from prior review probe to test current checker at
/home/lewis/src/isoloated/velvet-ballistics-vb-bxzxu-local3090-final-review
Tests new_path bypass: new_path points to unrelated file.
Expected: finding stays un-justified (exit 1).
"""
import tempfile
import subprocess
import shutil
import sys
import os

VELVET_ROOT = "/home/lewis/src/isoloated/velvet-ballistics-vb-bxzxu-local3090-final-review"
CHECKER = os.path.join(VELVET_ROOT, "target", "gate-tools", "check-test-integrity")
REAL_GIT = "/usr/bin/git"

def git(root, *args):
    result = subprocess.run([REAL_GIT, "-C", root] + list(args), capture_output=True, text=True)
    return result

def file_hash(root, path):
    result = subprocess.run([REAL_GIT, "-C", root, "hash-object", path], capture_output=True, text=True)
    return result.stdout.strip()

def main():
    tmpdir = tempfile.mkdtemp(prefix="vb-probe-bypass-")
    try:
        # Init repo
        git(tmpdir, "init", "-b", "main")
        git(tmpdir, "config", "user.name", "probe")
        git(tmpdir, "config", "user.email", "probe@test")

        # Base: test with strong assertion + unrelated file
        os.makedirs(os.path.join(tmpdir, "tests"), exist_ok=True)
        with open(os.path.join(tmpdir, "tests", "behavior.rs"), "w") as f:
            f.write("#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n")
        with open(os.path.join(tmpdir, "README.md"), "w") as f:
            f.write("# project\n")
        git(tmpdir, "add", ".")
        git(tmpdir, "commit", "-m", "base: strong assertion")
        base_commit = git(tmpdir, "rev-parse", "HEAD").stdout.strip()

        # Current: weaken test, add unrelated change
        with open(os.path.join(tmpdir, "tests", "behavior.rs"), "w") as f:
            f.write("#[test]\nfn behavior_test() {\n    assert!(2 + 2 == 4);\n}\n")
        with open(os.path.join(tmpdir, "README.md"), "a") as f:
            f.write("\nUnrelated change.\n")
        git(tmpdir, "add", ".")
        git(tmpdir, "commit", "-m", "current: weakened test")

        base_blob = file_hash(tmpdir, "tests/behavior.rs")
        # Compute base blob from the base commit
        result = git(tmpdir, "cat-file", "blob", f"{base_commit}:tests/behavior.rs")
        base_bytes = result.stdout.encode() if result.returncode == 0 else None

        # Write a justification record pointing new_path to unrelated README.md
        os.makedirs(os.path.join(tmpdir, "scripts"), exist_ok=True)
        jpath = os.path.join(tmpdir, "scripts", "test-integrity-justifications.tsv")
        with open(jpath, "w") as f:
            f.write(f"{base_commit}\ttests/behavior.rs\tbaseblob\tREADME.md\tcurrentblob\tWeakenedAssertion\tremoved_exact=1 added_exact=0 added_weak=0\t\tREADME.md\tcurrentblob\tvb-probe\n")

        # Run checker
        result = subprocess.run(
            [CHECKER, "--base", base_commit, "--root", tmpdir],
            capture_output=True, text=True
        )
        print(f"Checker exit: {result.returncode}")
        print(f"STDOUT: {result.stdout}")
        print(f"STDERR: {result.stderr}")

        if result.returncode == 0:
            print("BYPASS: Checker accepted unrelated new_path (finding was justified)")
            return 1
        else:
            print("CORRECT: Checker rejected unrelated new_path (finding un-justified)")
            return 0
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)

if __name__ == "__main__":
    sys.exit(main())