#!/usr/bin/env python3
"""
Validate that the three fuzz tasks in .moon/tasks/all.yml use consistent
target-dir flags and matching executable lookup paths.

Validation performed (all syntax/config only; no rebuild):
1. Each task has a shell script that passes `bash -n`
2. Build commands use `--target-dir`
3. Executable lookup paths use the same directory as build output
4. ASAN tasks (fuzz-smoke, fuzz-regress) share the same target dir
5. TSAN task (fuzz-tsan) uses a SEPARATE target dir
"""
import re
import subprocess
import sys

TASKS = ["fuzz-smoke", "fuzz-tsan", "fuzz-regress"]
YAML_FILE = ".moon/tasks/all.yml"


def extract_task_script(yaml_text, task_name):
    """Extract script text for a named task."""
    lines = yaml_text.split("\n")
    in_task = False
    in_script = False
    script_lines = []
    prev_indent = 0

    for i, line in enumerate(lines):
        if not in_task:
            if re.match(rf"^  {task_name}:$", line):
                in_task = True
                continue
            continue

        # Inside a task - detect next task at exactly 2-space indent
        if re.match(r"^  [a-z_-]+:", line):
            if in_script and script_lines:
                return "\n".join(script_lines)
            break

        if not in_script:
            m = re.match(r"^(\s+)script:\s*\|$", line)
            if m:
                in_script = True
                prev_indent = len(m.group(1))
            continue

        if in_script:
            if line.strip() == "":
                return "\n".join(script_lines)
            # Script content is indented one level deeper than script: field
            if len(line) >= prev_indent + 1 and line[prev_indent:prev_indent + 1] == " ":
                script_lines.append(line[prev_indent + 1:])
            else:
                return "\n".join(script_lines)

    if in_script and script_lines:
        return "\n".join(script_lines)
    return None


def validate_script_syntax(script, name):
    """Validate shell syntax using bash -n."""
    result = subprocess.run(
        ["bash", "-n", "-"],
        input=script,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return False, result.stderr
    return True, ""


def get_target_dir(script):
    """Extract --target-dir value from script."""
    match = re.search(r"--target-dir\s+\"?(\$PWD/)?([^\s\"]+)", script)
    if match:
        return match.group(2)
    return None


def get_binary_path(script):
    """Extract binary lookup path from script."""
    match = re.search(r'bin="([^\"]+)"', script)
    if match:
        return match.group(1)
    match = re.search(r'if \[ -x "([^\"]+)" \]', script)
    if match:
        return match.group(1)
    return None


def resolve_var_path(script, path):
    """Resolve shell variable references in a path string."""
    # Look for ${VAR}/pattern
    var_match = re.match(r'\$\{(\w+)\}/(.*)', path)
    if var_match:
        var_name = var_match.group(1)
        remainder = var_match.group(2)
        # Find the assignment
        assign_match = re.search(rf'{var_name}="?([^"\n]+)"?', script)
        if assign_match:
            var_value = assign_match.group(1).strip('"')
            return var_value + "/" + remainder
    return path


def paths_consistent(script, td, bp):
    """
    Check if binary path is under the target directory.
    Resolves variable references within the script.
    """
    if not td or not bp:
        return False

    # Resolve variable references in the binary path
    resolved_bp = resolve_var_path(script, bp)

    td_rel = td.lstrip("$PWD/")
    if td_rel.startswith("/"):
        td_rel = td_rel[1:]
    if resolved_bp.startswith("target/"):
        return resolved_bp.startswith(td_rel + "/")
    if resolved_bp.startswith("$PWD/"):
        bp_rel = resolved_bp[len("$PWD/"):]
        return bp_rel.startswith(td_rel + "/")
    return False


def main():
    with open(YAML_FILE, "r") as f:
        yaml_text = f.read()

    print(f"Validating {YAML_FILE}")
    print(f"Tasks: {', '.join(TASKS)}")
    print()

    errors = []
    target_dirs = {}

    for task in TASKS:
        print(f"--- {task} ---")
        script = extract_task_script(yaml_text, task)

        if not script:
            errors.append(f"{task}: could not extract script")
            print("  ERROR: could not extract script")
            continue

        # Validate shell syntax
        ok, err = validate_script_syntax(script, task)
        if ok:
            print("  ✓ shell syntax valid")
        else:
            errors.append(f"{task}: shell syntax error: {err}")
            print(f"  ✗ shell syntax: {err[:200]}")

        # Check target-dir
        td = get_target_dir(script)
        if td:
            target_dirs[task] = td
            print(f"  ✓ --target-dir: {td}")
        else:
            errors.append(f"{task}: missing --target-dir")
            print("  ✗ missing --target-dir")

        # Check binary path
        bp = get_binary_path(script)
        if bp:
            print(f"  ✓ binary path: {bp}")
            # Verify consistency
            if td and paths_consistent(script, td, bp):
                print(f"    ✓ binary path consistent with target-dir")
            elif td:
                errors.append(f"{task}: binary path {bp} inconsistent with target-dir {td}")
                print(f"    ✗ binary path inconsistent with target-dir")
        else:
            errors.append(f"{task}: missing binary path")
            print("  ✗ missing binary path")

        # Check for log redirection (v2 pathfix regression)
        if ">target/" in script and "build.log" in script:
            print("  ✓ build log redirection present")
        else:
            errors.append(f"{task}: missing build log redirection")
            print("  ✗ missing build log redirection")

        print()

    # Cross-task consistency
    print("--- Cross-task consistency ---")

    # ASAN tasks must share target dir
    asan_tasks = ["fuzz-smoke", "fuzz-regress"]
    asan_dirs = set()
    for t in asan_tasks:
        if t in target_dirs:
            asan_dirs.add(target_dirs[t])

    if len(asan_dirs) == 1:
        # Non-mutating lookup: list() instead of pop()
        shared_dir = list(asan_dirs)[0]
        print(f"  ✓ ASAN tasks share target dir: {shared_dir}")
    elif len(asan_dirs) > 1:
        errors.append(f"ASAN tasks use different target dirs: {asan_dirs}")
        print(f"  ✗ ASAN tasks use different target dirs: {asan_dirs}")

    # TSAN must use different dir
    if "fuzz-tsan" in target_dirs:
        tsan_dir = target_dirs["fuzz-tsan"]
        if len(asan_dirs) == 1 and tsan_dir != list(asan_dirs)[0]:
            print(f"  ✓ TSAN uses separate dir: {tsan_dir}")
        elif len(asan_dirs) == 1 and tsan_dir == list(asan_dirs)[0]:
            errors.append("TSAN and ASAN use the same target dir (race condition)")
            print("  ✗ TSAN and ASAN use the same target dir")
        else:
            errors.append("Cannot verify TSAN/ASAN dir separation: missing ASAN dirs")
            print("  ✗ Cannot verify TSAN/ASAN dir separation")

    # List actual targets
    result = subprocess.run(
        ["bash", "-c", "export PATH=/cache/cargo-shared/bin:${PATH}; cargo fuzz list"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        errors.append(f"cargo fuzz list failed (exit {result.returncode}): {result.stderr}")
        print(f"\n  cargo fuzz list FAILED (exit {result.returncode})")
        if result.stderr:
            print(f"  stderr: {result.stderr[:200]}")
    else:
        targets = result.stdout.strip().split("\n")
        print(f"\n  Actual fuzz targets: {len(targets)}")

    if errors:
        print()
        print("ERRORS:")
        for e in errors:
            print(f"  - {e}")
        return 1

    print()
    print("All validations passed (syntax and path consistency only)")
    return 0


if __name__ == "__main__":
    sys.exit(main())