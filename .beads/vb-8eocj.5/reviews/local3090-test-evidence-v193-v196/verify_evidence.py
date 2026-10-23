from pathlib import Path
import subprocess,json,hashlib,re,collections
ROOT=Path.cwd();OUT=Path('/home/lewis/Documents/Codex/2026-10-22/task/output/velvet-ballistics-review/test-evidence-local3090-v193-v196')
RECEIPTS=[('3de0bcfb9b95eaa99c172d8827a9a957e8d67660','.beads/vb-8eocj.5/native-qualification-v181-v193'),('747a8830613ca9f3a7a3fe513a32b5b45562d71a','.beads/vb-8eocj.5/native-repeat-kani-v194-v196')]
def git(*args):return subprocess.check_output(['git','--no-optional-locks',*args])
def get(c,p):return git('show',c+':'+p)
def sha(b):return hashlib.sha256(b).hexdigest()
raw={};manifest_checks=[]
for commit,folder in RECEIPTS:
 manifest=get(commit,folder+'/manifest.sha256.json');m=json.loads(manifest);details=[]
 for name,expected in m.items():
  assert Path(name).name==name
  data=get(commit,folder+'/'+name);actual=sha(data);assert actual==expected,(name,actual,expected)
  raw[(commit,name)]=data;details.append({'file':name,'bytes':len(data),'sha256':actual,'match':True})
 manifest_checks.append({'receipt':commit,'folder':folder,'manifest_sha256':sha(manifest),'entries':len(m),'all_match':True,'files':details})
 raw[(commit,'manifest.sha256.json')]=manifest
maps=[];trees={};needed={};unavailable=[]
def tree(c):
 if c not in trees:
  d={}
  for row in git('ls-tree','-rz',c).split(b'\0'):
   if not row:continue
   head,path=row.split(b'\t',1);mode,typ,oid=head.decode().split();d[path.decode()]=(mode,typ,oid)
  trees[c]=d
 return trees[c]
for (receipt,name),data in raw.items():
 if not name.endswith(('.source-before.json','.source-after.json')):continue
 d=json.loads(data);assert sha(json.dumps(d['files'],sort_keys=True).encode())==d['source_sha256']
 c=d['commit'];available=subprocess.run(['git','cat-file','-e',c+'^{commit}'],capture_output=True).returncode==0
 item={'receipt':receipt,'file':name,'commit':c,'file_count':len(d['files']),'source_sha256':d['source_sha256'],'aggregate_recomputed':True,'commit_available':available}
 if not available:
  assert c in {'3107f1e157916de3733a3e9dad3bcf60a8183a37','ffbe2db185c6826b04f76c327503060775fc4b3c'},c
  unavailable.append(c);item['limitation']='Recorded map valid; exact named commit blob identity unavailable. No fetch attempted.'
  comparison={'3107f1e157916de3733a3e9dad3bcf60a8183a37':'9aaa36d8ff29a9e8e3163cd8a1d145c0334cbcb8','ffbe2db185c6826b04f76c327503060775fc4b3c':'dd823160331dfac4280289541de419563cf0521f'}[c];item['fallback_context_only']=comparison
 else:comparison=c
 t=tree(comparison);scope={p for p,(mode,typ,oid) in t.items() if typ=='blob' and mode!='120000' and (p.startswith(('crates/','xtask/','scripts/','.moon/')) or p in ('Cargo.toml','Cargo.lock','rust-toolchain.toml','velvet-ballistics-MASTER.md'))}
 assert scope==set(d['files']),(name,'source scope paths differ',scope^set(d['files']))
 item['directory_symlink_excluded_by_native_is_file']=[p for p,(mode,typ,oid) in t.items() if mode=='120000' and p.startswith(('crates/','xtask/','scripts/','.moon/'))]
 for path,expected in d['files'].items():
  oid=t[path][2];needed[oid]=None
 item['_comparison']=comparison;item['_files']=d['files'];maps.append(item)
# Batch read exact Git blobs; avoid build/test execution and per-file process churn.
payload=git('cat-file','--batch',) if not needed else subprocess.check_output(['git','cat-file','--batch'],input=('\n'.join(needed)+'\n').encode())
pos=0;blob_hashes={}
for oid in needed:
 end=payload.index(b'\n',pos);head=payload[pos:end].decode().split();assert head[0]==oid and head[1]=='blob';size=int(head[2]);pos=end+1
 blob=payload[pos:pos+size];assert len(blob)==size;blob_hashes[oid]=sha(blob);pos+=size;assert payload[pos:pos+1]==b'\n';pos+=1
assert pos==len(payload)
for item in maps:
 t=tree(item.pop('_comparison'));files=item.pop('_files')
 for path,expected in files.items():assert blob_hashes[t[path][2]]==expected,(item['file'],path)
 item['matched_blob_count']=len(files);item['named_commit_blobs_verified']=item['commit_available']
pairs=[];gates=[]
for receipt,folder in RECEIPTS:
 for (r,n),b in raw.items():
  if r!=receipt or not n.endswith('_NATIVE.json'):continue
  d=json.loads(b);before=json.loads(raw[(r,n[:-5]+'.source-before.json')]);after=json.loads(raw[(r,n[:-5]+'.source-after.json')]);same=before['files']==after['files']
  gates.append({'receipt':r,'file':n,'argv':d['argv'],'exit_code':d['exit_code'],'seconds':d['seconds'],'timed_out':d['timed_out'],'commit_before':before['commit'],'commit_after':after['commit'],'source_unchanged':same,'source_sha256_before':before['source_sha256'],'source_sha256_after':after['source_sha256']})
  if 'FORMAT_APPLY' in n:
   changes=[p for p in before['files'] if before['files'][p]!=after['files'].get(p)];assert len(changes)==11
  else:assert same,n
r193=RECEIPTS[0][0];r196=RECEIPTS[1][0]
v193=json.loads(raw[(r193,'ROOT_WHOLE_NEXTEST_V193_NATIVE.json')]);s193=raw[(r193,'ROOT_WHOLE_NEXTEST_V193_NATIVE.stderr')].decode()
assert 'Starting 14728 tests across 309 binaries (27 tests skipped)' in s193
assert '14728 tests run: 14728 passed (21 slow), 27 skipped' in s193
assert v193['exit_code']==0 and not v193['timed_out']
before=json.loads(raw[(r193,'ROOT_CLI_BINARY_V193.before.json')]);after=json.loads(raw[(r193,'ROOT_CLI_BINARY_V193.after.json')]);binary_keys=['path','sha256','size','source_pin'];assert all(before[k]==after[k] for k in binary_keys)
assert before['sha256']=='d00ddb0d065d14be8c77029c89bcee8d6e76e9697e15183fdd07ffd70612e6dc'
assert 'CARGO_BIN_EXE_velvet-ballistics='+before['path'] in v193['argv']
v192=json.loads(raw[(r193,'ROOT_CLI_PINNED_BUILD_V192_NATIVE.json')]);assert v192['exit_code']==0
build_after=json.loads(raw[(r193,'ROOT_CLI_PINNED_BUILD_V192_NATIVE.source-after.json')]);assert build_after['commit']==before['source_pin'] and build_after['time_ns']<before['time_ns']<after['time_ns']
stdout=raw[(r196,'ROOT_REPEAT_KANI_V196_NATIVE.stdout')].decode();v196=json.loads(raw[(r196,'ROOT_REPEAT_KANI_V196_NATIVE.json')]);assert v196['exit_code']==0 and not v196['timed_out']
checks=[]
for block in re.split(r'(?=^Check \d+:)',stdout,flags=re.M):
 m=re.match(r'Check (\d+): ([^\n]+)',block)
 if not m:continue
 status=re.search(r'^\s*- Status: ([A-Z_]+)',block,re.M);assert status
 desc=re.search(r'^\s*- Description: (.*)',block,re.M)
 checks.append({'id':int(m.group(1)),'property':m.group(2),'status':status.group(1),'description':desc.group(1) if desc else None})
counts=dict(collections.Counter(c['status'] for c in checks));assert counts=={'UNREACHABLE':15,'SUCCESS':547,'SATISFIED':12},counts
assert len(checks)==574 and len({c['id'] for c in checks})==574
assert '0 of 562 failed (15 unreachable)' in stdout and '12 of 12 cover properties satisfied' in stdout and 'VERIFICATION:- SUCCESSFUL' in stdout
assert 'Complete - 1 successfully verified harnesses, 0 failures, 1 total.' in stdout
(out_checks:=OUT/'v196-all-check-statuses.json').write_text(json.dumps(checks,indent=2)+'\n')
(OUT/'v196-raw.stdout').write_bytes(raw[(r196,'ROOT_REPEAT_KANI_V196_NATIVE.stdout')])
parent193=git('rev-parse',r193+'^').decode().strip();assert parent193=='daa75236facecac24c30be18ecb733d604fbf5d8'
crates193=git('rev-parse',r193+':crates').decode().strip();assert crates193==git('rev-parse',parent193+':crates').decode().strip()
refs196=['dd823160331dfac4280289541de419563cf0521f',r196]
crates196={r:git('rev-parse',r+':crates').decode().strip() for r in refs196};assert len(set(crates196.values()))==1
assert git('rev-parse',r196+'^').decode().strip()==refs196[0]
imports=json.loads(raw[(r196,'imported-review-manifest.json')]);assert len(imports)==31
for d in imports:
 data=get(r196,d['path']);assert len(data)==d['bytes'] and sha(data)==d['sha256'];assert data==get(d['commit'],d['path'])
result={'review_scope':'Receipt integrity/source identity/binary provenance/verifier raw results only; no test/build/proof rerun','manifest_checks':manifest_checks,'source_maps':maps,'source_maps_checked':len(maps),'git_blob_versions_hashed':len(blob_hashes),'unavailable_named_commits':sorted(set(unavailable)),'native_gates':gates,'v193_raw_aggregate':{'run':14728,'passed':14728,'skipped':27,'binaries':309,'slow':21,'exit_code':0,'wrapper_seconds':527.295,'nextest_summary_seconds':498.881,'source':parent193,'source_map_sha256':build_after['source_sha256'],'crates_tree':crates193,'individual_pass_names_available':False,'skip_identities_available':False,'historical119_by_name_closure_established':False},'binary_provenance':{'recorded_before_after_identical':True,'metadata':{k:before[k] for k in binary_keys},'explicit_selected_path_matches':True,'build_source_matches':True,'build_precedes_captured_binary':True,'limitation':'Historic binary image not stored here; verified captured hash/size/path/source/build/selection evidence, not a reproducible-build proof.'},'v196_raw_result':{'checks_total_excluding_covers':562,'success':547,'unreachable':15,'failed':0,'covers':12,'covers_satisfied':12,'harnesses_successful':1,'exit_code':0,'seconds':806.705,'verification_seconds_from_stdout':804.8345,'statuses_counted_from_all574blocks':True,'unsupported_construct_inventory_retained':True,'source_trees':crates196,'receipt_parent_verified_as_dd':True,'named_post_run_ffbe_commit_available':False,'post_run_file_map_matches_dd_blobs':True,'imported_review_files_verified':31},'limits':['V193 output is aggregate; not historical119 by-name closure or identity of27skips.','3107 and post-run ffbe named commits unavailable locally: exact named-object identity/ffbe-parent unverified; recorded maps independently match verified9aaa/dd respectively. No fetch of these refs attempted.','V181 formatter applies11changes: raw0 differs from wrapper125 source-drift policy.','V187 timeout124 no verdict remains historical; V196 does not rewrite it.','Kani is one bounded raw-helper harness; not general runtime/admission/F1/canonicalCI/main acceptance.','Kani unsupported inventory/backend model messages retained; no entire-crate support claim.']}
(OUT/'mechanical-integrity.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'manifests':[(d['receipt'],d['entries']) for d in manifest_checks],'source_maps':len(maps),'unique_git_blobs':len(blob_hashes),'unavailable':sorted(set(unavailable)),'v193':result['v193_raw_aggregate'],'v196':result['v196_raw_result']},indent=2))
