#!/usr/bin/env python3
import sys
import subprocess
import time
import json
import os

def main():
    if len(sys.argv) < 4:
        print("Usage: python3 vb-run-gate.py <timeout> <evidence-dir> <command...>")
        sys.exit(1)

    timeout = int(sys.argv[1])
    evidence_dir = sys.argv[2]
    cmd = sys.argv[3:]

    os.makedirs(evidence_dir, exist_ok=True)

    timestamp = time.strftime("%Y%m%d-%H%M%S")
    prefix = "vb-8li6d-{}".format(timestamp)

    stdout_file = os.path.join(evidence_dir, "{}-stdout.log".format(prefix))
    stderr_file = os.path.join(evidence_dir, "{}-stderr.log".format(prefix))
    metadata_file = os.path.join(evidence_dir, "{}-metadata.json".format(prefix))

    print("Evidence prefix: {}".format(prefix))
    print("Running: {}".format(" ".join(cmd)))

    start = time.time()

    with open(stdout_file, "w") as out_f:
        with open(stderr_file, "w") as err_f:
            try:
                result = subprocess.run(
                    cmd,
                    stdout=out_f,
                    stderr=err_f,
                    timeout=timeout,
                )
                exit_code = result.returncode
            except subprocess.TimeoutExpired as e:
                exit_code = 124
                print("TIMEOUT after {}s".format(timeout))

    elapsed = time.time() - start

    metadata = {
        "prefix": prefix,
        "command": cmd,
        "timestamp": timestamp,
        "elapsed_seconds": elapsed,
        "exit_code": exit_code,
        "stdout_file": stdout_file,
        "stderr_file": stderr_file,
    }

    with open(metadata_file, "w") as f:
        json.dump(metadata, f, indent=2)

    print("Elapsed: {:.2f}s".format(elapsed))
    print("Exit code: {}".format(exit_code))
    print("Stdout: {}".format(stdout_file))
    print("Stderr: {}".format(stderr_file))
    print("Metadata: {}".format(metadata_file))

    sys.exit(exit_code)

if __name__ == "__main__":
    main()
