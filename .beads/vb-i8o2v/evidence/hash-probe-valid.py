#!/usr/bin/env python3
"""Valid hash-binding probe for scripts/check-test-integrity.rs (vb-i8o2v).

Exercises the four hash-binding behaviors of the CURRENT checker using real
40-hex blob hashes, the real /usr/bin/git, and temp git repos:

  1. unrelated new_path      -> finding stays un-justified (exit 1)
  2. new_path=ABSENT (old exists) -> exit 1
  3. exact same path (correct record) -> exit 0
  4. subsequently changed source    -> exit 1

Each variation first establishes the baseline (correct record -> exit 0),
then applies the variation. The script exits 0 ONLY if all cases pass.

Usage:
  python3 hash-probe-valid.py /abs/path/to/check-test-integrity
"""
import os
import shutil
import subprocess
import sys
import tempfile

REAL_GIT = "/usr/bin/git"
TEST_PATH = "crates/workspace_tests/tests/behavior.rs"
REVIEW_PATH = ".beads/vb-probe/test-review.md"
REVIEW_TEXT = "# Probe review artifact\n"
BASE_TEXT = "#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n"
WEAK_TEXT = "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n"
WEAK_TEXT2 = "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(5).is_some());\n}\n"
FINDING_DETAIL = "removed_exact=1 added_exact=0 added_weak=1"
FINDING_LINE = "WeakenedAssertion|" + TEST_PATH + "|" + FINDING_DETAIL


def git(cwd, *args):
    return subprocess.run([REAL_GIT, "-C", cwd] + list(args),
                          capture_output=True, text=True)


def git_ok(cwd, *args):
    r = git(cwd, *args)
    if r.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {r.stderr}")
    return r.stdout.strip()


def write(rel, text, cwd):
    full = os.path.join(cwd, rel)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, "w") as fh:
        fh.write(text)


def setup_repo():
    cwd = tempfile.mkdtemp(prefix="vb-hash-probe-")
    git_ok(cwd, "init", "-b", "main")
    git_ok(cwd, "config", "user.name", "probe")
    git_ok(cwd, "config", "user.email", "probe@example.com")
    write(TEST_PATH, BASE_TEXT, cwd)
    git_ok(cwd, "add", "-A")
    git_ok(cwd, "commit", "-m", "base")
    base_commit = git_ok(cwd, "rev-parse", "HEAD")
    # Working-tree current state: weakened test + review artifact (untracked).
    write(TEST_PATH, WEAK_TEXT, cwd)
    write(REVIEW_PATH, REVIEW_TEXT, cwd)
    base_blob = git_ok(cwd, "rev-parse", "--verify", f"{base_commit}:{TEST_PATH}")
    new_blob = git_ok(cwd, "hash-object", TEST_PATH)
    review_blob = git_ok(cwd, "hash-object", REVIEW_PATH)
    return cwd, base_commit, base_blob, new_blob, review_blob


def write_tsv(cwd, base_commit, old_path, old_blob, new_path, new_blob,
              review_blob, replacement="-"):
    tsv = (
        f"{base_commit}\t{old_path}\t{old_blob}\t{new_path}\t{new_blob}"
        f"\tWeakenedAssertion\t{FINDING_DETAIL}\t{replacement}"
        f"\t{REVIEW_PATH}\t{review_blob}\tvb-probe\n"
    )
    write("scripts/test-integrity-justifications.tsv", tsv, cwd)


def run_checker(binary, cwd, base_commit):
    r = subprocess.run([binary, "--base", base_commit], cwd=cwd,
                       capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def expect_pass(code, out, err):
    if code != 0:
        return False, f"expected exit 0, got {code}; stderr={err!r}"
    if "test integrity: PASS" not in out:
        return False, f"missing PASS marker; stdout={out!r}"
    if "JUSTIFIED|" + FINDING_LINE not in out:
        return False, f"missing JUSTIFIED line; stdout={out!r}"
    return True, ""


def expect_fail(code, out, err):
    if code != 1:
        return False, f"expected exit 1, got {code}; stderr={err!r}"
    if "test integrity: FAIL" not in err:
        return False, f"missing FAIL marker; stderr={err!r}"
    if FINDING_LINE not in err:
        return False, f"missing finding line; stderr={err!r}"
    return True, ""


def case(name, mutate, expect_fn, binary):
    cwd = base_commit = base_blob = new_blob = review_blob = None
    try:
        cwd, base_commit, base_blob, new_blob, review_blob = setup_repo()
        # Baseline: correct record (exact same path) must justify (exit 0).
        write_tsv(cwd, base_commit, TEST_PATH, base_blob, TEST_PATH, new_blob,
                  review_blob)
        code, out, err = run_checker(binary, cwd, base_commit)
        ok, msg = expect_pass(code, out, err)
        if not ok:
            print(f"FAIL {name}: baseline: {msg}")
            return False
        # Apply the variation and assert the exact un-justified finding.
        code, out, err = mutate(cwd, base_commit, base_blob, new_blob,
                                review_blob, binary)
        ok, msg = expect_fn(code, out, err)
        if not ok:
            print(f"FAIL {name}: variation: {msg}")
            return False
        print(f"PASS {name}")
        return True
    finally:
        if cwd:
            shutil.rmtree(cwd, ignore_errors=True)


def mutate_unrelated(cwd, base_commit, base_blob, new_blob, review_blob, binary):
    write("README.md", "# Unrelated\n", cwd)
    unrelated_blob = git_ok(cwd, "hash-object", "README.md")
    write_tsv(cwd, base_commit, TEST_PATH, base_blob, "README.md",
              unrelated_blob, review_blob)
    return run_checker(binary, cwd, base_commit)


def mutate_absent(cwd, base_commit, base_blob, new_blob, review_blob, binary):
    write_tsv(cwd, base_commit, TEST_PATH, base_blob, "ABSENT", "ABSENT",
              review_blob)
    return run_checker(binary, cwd, base_commit)


def mutate_changed(cwd, base_commit, base_blob, new_blob, review_blob, binary):
    # Keep the record's new_blob (stale) but change the actual source.
    write(TEST_PATH, WEAK_TEXT2, cwd)
    return run_checker(binary, cwd, base_commit)


def case_exact_same_path(binary):
    cwd = None
    try:
        cwd, base_commit, base_blob, new_blob, review_blob = setup_repo()
        write_tsv(cwd, base_commit, TEST_PATH, base_blob, TEST_PATH, new_blob,
                  review_blob)
        code, out, err = run_checker(binary, cwd, base_commit)
        ok, msg = expect_pass(code, out, err)
        if not ok:
            print(f"FAIL exact-same-path: {msg}")
            return False
        print("PASS exact-same-path")
        return True
    finally:
        if cwd:
            shutil.rmtree(cwd, ignore_errors=True)


def main():
    if len(sys.argv) != 2:
        print("usage: hash-probe-valid.py /abs/path/to/check-test-integrity")
        return 2
    binary = sys.argv[1]
    if not os.path.isfile(binary):
        print(f"FAIL: checker binary not found: {binary}")
        return 2
    if not os.path.isfile(REAL_GIT):
        print(f"FAIL: real git not found: {REAL_GIT}")
        return 2

    results = [
        case_exact_same_path(binary),
        case("unrelated-new-path", mutate_unrelated, expect_fail, binary),
        case("absent-while-old-exists", mutate_absent, expect_fail, binary),
        case("subsequent-source-change", mutate_changed, expect_fail, binary),
    ]
    if all(results):
        print("hash-probe: ALL PASS")
        return 0
    print("hash-probe: FAILURES PRESENT")
    return 1


if __name__ == "__main__":
    sys.exit(main())
