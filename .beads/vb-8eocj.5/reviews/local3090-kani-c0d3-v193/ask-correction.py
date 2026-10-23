from pathlib import Path
import json,urllib.request,time
p=Path(__file__).parent; req=json.loads((p/'correction-request.json').read_text());t=time.time()
try:
 with urllib.request.urlopen(urllib.request.Request('http://127.0.0.1:11001/v1/chat/completions',json.dumps(req).encode(),{'Content-Type':'application/json'}),timeout=600) as r:d=json.load(r)
 (p/'correction-response.json').write_text(json.dumps(d));c=d['choices'][0];s={'finish_reason':c['finish_reason'],'seconds':time.time()-t,'usage':d.get('usage')};(p/'correction-status.json').write_text(json.dumps(s));print(json.dumps(s))
 if c['finish_reason']=='stop':(p/'correction-report.txt').write_text(c['message'].get('content') or '')
except Exception as e:
 s={'error':str(e),'seconds':time.time()-t};(p/'correction-status.json').write_text(json.dumps(s));print(json.dumps(s))
