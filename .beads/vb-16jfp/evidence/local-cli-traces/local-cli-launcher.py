import argparse,json,subprocess,sys
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument("gpu",choices=["5090","3090"]);p.add_argument("workspace");p.add_argument("prompt");p.add_argument("prefix");p.add_argument("--resume")
a=p.parse_args();prefix=Path(a.prefix);provider="local_qwen_"+a.gpu;port="11002" if a.gpu=="5090" else "11003"
cmd=["codex","exec","--ignore-user-config","-c",f'model_provider="{provider}"',"-c",f'model_providers.{provider}={{name="Local RTX {a.gpu}",base_url="http://127.0.0.1:{port}/v1",wire_api="responses",requires_openai_auth=false}}',"-c",'model_reasoning_effort="high"',"-c",'model_reasoning_summary="none"',"-c",'web_search="disabled"',"-c",'approval_policy="never"',"-c","features.memories=false","-c","features.apps=false","-c","model_context_window=262144","-m","qwen3.8-27b-uncensored-"+a.gpu,"-s","danger-full-access","-C",a.workspace]
if a.resume:cmd += ["resume"]
cmd += ["--json","-o",str(prefix)+"-final.txt"]
if a.resume:cmd += [a.resume]
cmd += ["-"]
Path(str(prefix)+"-invocation.json").write_text(json.dumps({"command":cmd,"prompt":a.prompt,"workspace":a.workspace,"provider":provider,"cloud_fallback":False},indent=2))
with open(a.prompt,"rb") as inp,open(str(prefix)+"-events.jsonl","wb") as out,open(str(prefix)+"-stderr.log","wb") as err:
 r=subprocess.run(cmd,stdin=inp,stdout=out,stderr=err,cwd=a.workspace)
Path(str(prefix)+"-exit-code").write_text(str(r.returncode)+"\n")
print(json.dumps({"gpu":a.gpu,"exit":r.returncode,"prefix":str(prefix)}));sys.exit(r.returncode)
