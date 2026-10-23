#!/usr/bin/env python3
"""Summarize independent process medians; preserve all within-process samples."""
import csv,json,pathlib,statistics as st
ROOT=pathlib.Path(__file__).resolve().parents[1];OUT=ROOT/'results'
rows=[];reports={}
for p in sorted((OUT/'raw').glob('*.json')):
 d=json.loads(p.read_text());cell=p.stem.rsplit('-',1)[0];reports.setdefault(cell,[]).append(d)
summary={}
for cell,rr in sorted(reports.items()):
 cases={}
 for r in rr:
  for m in r['measurements']:cases.setdefault(m['case'],[]).append(m['median_ns'])
 ms={name:{'median_ns':st.median(v),'process_medians_ns':v,'min_process_median_ns':min(v),'max_process_median_ns':max(v)} for name,v in cases.items()}
 summary[cell]={'processes':len(rr),'measurements':ms,'instantiate_median_ns':st.median([st.median(r['instantiate_ns']) for r in rr]),'first_scalar_median_ns':st.median([st.median(r['first_scalar_ns']) for r in rr]),'compile_median_ms':st.median([r['compile_ms'] for r in rr])}
 for name,m in ms.items():rows.append([cell,name,len(rr),m['median_ns'],m['min_process_median_ns'],m['max_process_median_ns']])
(OUT/'timings.json').write_text(json.dumps(summary,indent=2)+'\n')
with (OUT/'timings.csv').open('w') as f:
 w=csv.writer(f,lineterminator='\n');w.writerow(['cell','case','processes','median_ns','min_process_median_ns','max_process_median_ns']);w.writerows(rows)
text=['# All measured cases','', 'Nanoseconds per complete component call. Median of the 15-batch medians from each independent process; brackets are the smallest/largest process medians, not a confidence interval.','']
for engine in ['boa','porffor']:
 cells=[c for c in summary if c.startswith(engine+'-')]
 if not cells:continue
 text += ['## '+engine.title(),'','| Case | '+' | '.join(c for c in cells)+' |','|---|'+'---:|'*len(cells)]
 for case in summary[cells[0]]['measurements']:
  values=[]
  for c in cells:
   m=summary[c]['measurements'][case];values.append(f"{m['median_ns']:,.0f} [{m['min_process_median_ns']:,.0f}–{m['max_process_median_ns']:,.0f}]")
  text += ['| '+case+' | '+' | '.join(values)+' |']
 text+=['']
(OUT/'TABLES.md').write_text('\n'.join(text).rstrip()+'\n')
print('Wrote results/timings.json, timings.csv, TABLES.md')
