#!/usr/bin/env python3
"""Optional A controls, separate from the unmodified-generator main comparison."""
import pathlib,shutil,subprocess,os,re,sys
from build import ROOT,GEN,BASE,LIBS,run,rust
os.chdir(ROOT)
# Same Boa source and native ABI, only generated diagnostic statements removed.
g=ROOT/'build/generated-nolog';shutil.copytree(GEN,g,dirs_exist_ok=True)
run(['node','scripts/ablate.mjs',g/'wit.js'])
rust('boa-a',[f'-Clink-arg=-Wl,--component-type={ROOT}/wit/world.wit'],{'BINDINGS_DIR':str(g)})
d=ROOT/'build/boa-a-nolog';d.mkdir(exist_ok=True);shutil.copyfile(ROOT/'target/wasm32-wasip2/release/boa_a.wasm',d/'component.wasm')
for mode in ([] if '--boa-only' in sys.argv else ['nolog','native-utf8']):
 d=ROOT/'build'/f'porffor-a-{mode}';d.mkdir(exist_ok=True)
 run(['target/release/generate','wit','--source','packages/app/app.js','--world','bench','--out-dir',d])
 run(['node','scripts/ablate.mjs',d/'wit.js',mode])
 run(['node_modules/.bin/esbuild',d/'main.js','--bundle','--format=esm','--platform=neutral',f'--outfile={d}/bundle.js','--log-level=warning',*(d/'esbuild-aliases.txt').read_text().splitlines()])
 run(['node','upstream/porffor/runtime/index.js','c','--module',d/'bundle.js','-o',d/'app.c'])
 if mode=='native-utf8':
  source=(ROOT/'native/porffor/engine.c').read_text()
  source=source[source.index('ev engine_string('):source.index('ev engine_bytes(')]
  source=source.replace('ev engine_string(', 'static jsval utf8_from_bytes(').replace('return keep(porf_box(p,ascii?195:67));','return porf_box(p,ascii?195:67);')
  source=source.replace('size_t engine_to_string(ev x,','static size_t utf8_to_bytes(jsval v,').replace('jsval v=evs[x];assert(v.type==67||v.type==195);','')
  source+='''
static jsval wit_native_decode(jsval b) {u32 n=*(u32*)(MEM+(u32)b.val);return utf8_from_bytes(WIT_TA_DATA(b),n);}
static int32_t wit_native_encode(jsval s,int32_t *n) {uint8_t *tmp;*n=utf8_to_bytes(s,&tmp);uint32_t p=wit_js_realloc(*n);memcpy((void*)(uintptr_t)p,tmp,*n);free(tmp);return p;}
'''
  with (d/'app.c').open('a') as f:f.write('#include <stdbool.h>\n'+source)
 run([*BASE,'-mexec-model=reactor',d/'app.c',f'-Wl,--component-type={ROOT}/wit/world.wit',*LIBS,'-o',d/'component.wasm'])
for cell in (['boa-a-nolog'] if '--boa-only' in sys.argv else ['boa-a-nolog','porffor-a-nolog','porffor-a-native-utf8']):
 run(['wasm-tools','validate',f'build/{cell}/component.wasm']);run(['wasm-tools','strip','--all',f'build/{cell}/component.wasm','-o',f'build/{cell}/stripped.wasm'])
