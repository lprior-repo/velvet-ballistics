from pathlib import Path
import subprocess,os,time,json
root=Path.cwd();b=root/'.beads/vb-9pycz';env=os.environ.copy();env.update(RUSTC_WRAPPER='',CARGO_BUILD_JOBS='2')
checks=[('runtime-shutdown',['cargo','test','-p','vb_runtime','--lib','shutdown']),('storage-queue',['cargo','test','-p','vb_storage','--lib','queue::']),('ipc-library',['cargo','test','-p','vb_ipc','--lib']),('cli-shared-service',['cargo','test','-p','velvet-ballistics','--bin','velvet-ballistics','ipc_serve::tests'])]
author=[json.loads(l) for l in open('.beads/vb-aaqp7/commands.jsonl')]
checks.extend((label,next(r['argv'] for r in author if r['label']==label)) for label in ['shutdown-migrated-workspace-tests','shutdown-source-lint','shutdown-format-check'])
for label,args in checks:
 t=time.monotonic()
 with (b/'raw'/f'{label}.stdout').open('w') as out,(b/'raw'/f'{label}.stderr').open('w') as err:r=subprocess.run(['timeout','--kill-after=10','180',*args],env=env,stdout=out,stderr=err)
 row=dict(label=label,argv=args,cwd=str(root),exit_code=r.returncode,seconds=round(time.monotonic()-t,3),source='6a5a4d4e9861e6dc989362b496de1b6d47b4b939');print(json.dumps(row),flush=True)
 with (b/'commands.jsonl').open('a') as f:f.write(json.dumps(row)+'\n')
