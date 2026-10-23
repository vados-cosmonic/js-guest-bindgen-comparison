#!/usr/bin/env python3
"""Sequential, CPU-pinned measurements; record raw process-level repetitions."""
import argparse, gzip, hashlib, json, os, pathlib, platform, subprocess, time, re
ROOT=pathlib.Path(__file__).resolve().parents[1];os.chdir(ROOT)
def cmd(args):return subprocess.check_output(args,text=True).strip()
def sizes():
    result={}
    for cell in [f'{e}-{a}' for e in ['boa','porffor'] for a in 'abc'] + [x for x in ['boa-a-nolog','porffor-a-nolog','porffor-a-native-utf8'] if (ROOT/'build'/x/'component.wasm').exists()]:
        d=ROOT/'build'/cell;p=d/'component.wasm';s=d/'stripped.wasm'
        subprocess.run(['wasm-tools','validate',str(p)],check=True)
        subprocess.run(['wasm-tools','strip','--all',str(p),'-o',str(s)],check=True)
        raw=p.read_bytes();data=s.read_bytes();dump=cmd(['wasm-tools','objdump',str(s)])
        (ROOT/'results'/f'{cell}-sections.txt').write_text(dump+'\n')
        def sections(kind):
            return sum(int(m[1]) for line in dump.splitlines() if (m:=re.match(r'^\s*'+re.escape(kind)+r'\s*\|[^|]+\|\s*(\d+) bytes',line)))
        result[cell]={'raw_bytes':len(raw),'stripped_bytes':len(data),'gzip9_bytes':len(gzip.compress(data,compresslevel=9,mtime=0)),'sha256':hashlib.sha256(raw).hexdigest(),'stripped_sha256':hashlib.sha256(data).hexdigest(),'code_section_bytes':sections('code'),'data_section_bytes':sections('data'),'core_module_count':len(re.findall(r'^\s*module\s+\|',dump,re.M))}
    # Generated sources and separately built libraries give context for totals.
    artifacts=['build/generated/wit.js','build/generated/bench.c','build/generated/world.so','build/boa-c/libengine.so','build/porffor-c/libengine.so','build/porffor-a/bundle.js','build/porffor-b/bundle.js','build/porffor-a/app.c','build/porffor-b/app.c']
    extra={}
    for name in artifacts:
        p=ROOT/name;item={'bytes':p.stat().st_size}
        if p.suffix=='.so':
            stripped=p.with_suffix('.stripped.so');subprocess.run(['wasm-tools','strip','--all',str(p),'-o',str(stripped)],check=True);item['stripped_bytes']=stripped.stat().st_size
        extra[name]=item
    (ROOT/'results/sizes.json').write_text(json.dumps({'components':result,'artifacts':extra},indent=2)+'\n')
def main():
    p=argparse.ArgumentParser();p.add_argument('--rounds',type=int,default=3);p.add_argument('--sizes-only',action='store_true');p.add_argument('--cells',nargs='*');args=p.parse_args()
    pathlib.Path('results/raw').mkdir(parents=True,exist_ok=True)
    sizes()
    if args.sizes_only:return
    allowed=sorted(os.sched_getaffinity(0));cpu=allowed[0]
    environment={'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'platform':platform.platform(),'cpu_model':next((x.split(':',1)[1].strip() for x in pathlib.Path('/proc/cpuinfo').read_text().splitlines() if x.startswith('model name')),'unknown'),'allowed_cpus':allowed,'pinned_cpu':cpu,'rust':cmd(['rustc','--version']),'node':cmd(['node','--version']),'wasm_tools':cmd(['wasm-tools','--version']),'rounds':args.rounds,'runner_sha256':hashlib.sha256(pathlib.Path('target/host/release/runner').read_bytes()).hexdigest(),'method':'15 adaptive warm batches per process, 9 fresh instances for cold calls, automatic post-return included, serial process runs, rotated order'}
    pathlib.Path('results/environment.json').write_text(json.dumps(environment,indent=2)+'\n')
    cells=args.cells or ['boa-a','porffor-c','boa-b','porffor-a','boa-c','porffor-b'] + [x for x in ['boa-a-nolog','porffor-a-nolog','porffor-a-native-utf8'] if (ROOT/'build'/x/'component.wasm').exists()]
    for i in range(args.rounds):
        order=cells[i:]+cells[:i]
        for cell in order:
            print(f'round {i+1}: {cell}',flush=True)
            with open(f'results/raw/{cell}-{i+1}.log','w') as log:
                subprocess.run(['target/host/release/runner',f'build/{cell}/stripped.wasm',f'results/raw/{cell}-{i+1}.json'],stdout=log,stderr=subprocess.STDOUT,check=True,preexec_fn=lambda:os.sched_setaffinity(0,{cpu}))
if __name__=='__main__':main()
