import json, os, subprocess, sys, time
from pathlib import Path
command=sys.argv[2:]
env=os.environ.copy(); env["CARGO_BUILD_JOBS"]="2"
start=time.time()
result=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env)
p=Path(".beads/vb-7r7jv/evidence")
(p/(sys.argv[1]+".stdout")).write_bytes(result.stdout)
(p/(sys.argv[1]+".stderr")).write_bytes(result.stderr)
(p/(sys.argv[1]+".json")).write_text(json.dumps({"command":command,"cwd":str(Path.cwd()),"exit":result.returncode,"duration_seconds":time.time()-start,"cargo_build_jobs":2},indent=2)+"\n")
sys.stdout.buffer.write(result.stdout); sys.stderr.buffer.write(result.stderr)
print("ACTUAL_EXIT="+str(result.returncode))
sys.exit(result.returncode)
