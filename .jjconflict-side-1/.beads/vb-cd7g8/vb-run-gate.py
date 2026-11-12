#!/usr/bin/env python3
import json, subprocess, time, sys, os

def run(seconds, output_dir, name, command, args):
    cmd = [command] + args
    start = time.time()
    result = {"command": cmd}
    try:
        proc = subprocess.run(cmd, timeout=seconds, capture_output=True, text=True)
        elapsed = time.time() - start
        result.update({"exit_code": proc.returncode, "elapsed_seconds": elapsed, "stdout": proc.stdout, "stderr": proc.stderr})
    except subprocess.TimeoutExpired as e:
        elapsed = time.time() - start
        result.update({"exit_code": -1, "elapsed_seconds": elapsed, "stdout": e.stdout or "", "stderr": e.stderr or "", "timed_out": True})
    os.makedirs(output_dir, exist_ok=True)
    with open(os.path.join(output_dir, f"{name}.json"), "w") as f:
        json.dump(result, f, indent=2)
    with open(os.path.join(output_dir, f"{name}.stdout"), "w") as f:
        f.write(result["stdout"])
    with open(os.path.join(output_dir, f"{name}.stderr"), "w") as f:
        f.write(result["stderr"])
    return result["exit_code"]

if __name__ == "__main__":
    seconds = int(sys.argv[1])
    output_dir = sys.argv[2]
    name = sys.argv[3]
    command = sys.argv[4]
    args = sys.argv[5:]
    exit(run(seconds, output_dir, name, command, args))
