from pathlib import Path
import json,urllib.request,time
out=Path(__file__).parent
request={"model":"qwen3.8-27b-uncensored","reasoning_effort":"medium","messages":[{"role":"system","content":"You are a independent local Rust static reviewer. Cite actual supplied source and APIs, never invent execution or proof evidence."},{"role":"user","content":(out/"review-prompt.txt").read_text()}],"max_tokens":14000,"temperature":0.05}
(out/"review-request.json").write_text(json.dumps(request)); start=time.time()
try:
 with urllib.request.urlopen(urllib.request.Request("http://127.0.0.1:11001/v1/chat/completions",json.dumps(request).encode(),{"Content-Type":"application/json"}),timeout=1200) as response: result=json.load(response)
 (out/"review-response.json").write_text(json.dumps(result));choice=result["choices"][0]; content=choice["message"].get("content")or""
 if content.strip().startswith("```"):content=content.strip().split("\n",1)[1].rsplit("```",1)[0]
 status={"finish_reason":choice["finish_reason"],"seconds":time.time()-start,"usage":result.get("usage"),"content_bytes":len(content.encode())}
 if choice["finish_reason"]=="stop" and content.strip(): (out/"review-report.json").write_text(content)
 (out/"review-status.json").write_text(json.dumps(status));print(json.dumps(status))
except Exception as error:
 status={"error":str(error),"body":error.read().decode() if hasattr(error,"read") else None,"seconds":time.time()-start};(out/"review-status.json").write_text(json.dumps(status));print(json.dumps(status))
