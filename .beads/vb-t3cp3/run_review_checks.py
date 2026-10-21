import os, subprocess, json, time
from pathlib import Path
root=Path.cwd(); base=root/'.beads/vb-t3cp3'; env=os.environ.copy();env.update(RUSTC_WRAPPER='',CARGO_BUILD_JOBS='2')
checks=[('ipc-reactor', ['cargo','test','-p','vb_ipc','--lib','server::reactor_tests']),('storage-queue',['cargo','test','-p','vb_storage','--lib','queue::']),('cli-service',['cargo','test','-p','velvet-ballistics','--bin','velvet-ballistics','ipc_serve::tests']),('ipc-acceptance',['cargo','test','-p','velvet-ballistics-workspace-tests','--test','vb_te1i_binary_ipc_acceptance'])]
for label,args in checks:
 t=time.monotonic()
 with (base/'raw'/f'{label}.stdout').open('w') as out,(base/'raw'/f'{label}.stderr').open('w') as err:
  result=subprocess.run(['timeout','--kill-after=10','180',*args],stdout=out,stderr=err,env=env)
 row=dict(label=label,argv=args,cwd=str(root),source='862543ed44c354846235f6676675ed86b3f19add',exit_code=result.returncode,seconds=round(time.monotonic()-t,3))
 with (base/'commands.jsonl').open('a') as file:file.write(json.dumps(row)+'\n')
 print(json.dumps(row),flush=True)
