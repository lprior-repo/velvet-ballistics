#!/usr/bin/env python3
import subprocess
p = subprocess.Popen(
    ["cargo", "test", "-p", "vb_runtime", "--lib",
     "--",
     "shard::lifecycle_tests::strict_policy_sequence::successful_strict_policy_port",
     "shard::lifecycle_tests::strict_policy_sequence::typed_strict_policy_port_failure",
     "shard::lifecycle_tests::strict_policy_sequence::overflow_at_u64_max",
     "shard::lifecycle_tests::strict_policy_sequence::wrong_ticket_rejected"],
    cwd="/home/lewis/src/isoloated/vb-z7ky4-shard-tests-v3",
    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
)
p.wait(timeout=300)
out, err = p.communicate()
print("STDOUT:")
print(out[-3000:])
if err:
    print("STDERR:")
    print(err[-2000:])