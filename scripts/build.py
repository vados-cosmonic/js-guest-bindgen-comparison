#!/usr/bin/env python3
"""Reproducible six-cell build; all child processes execute in this repository."""
import argparse, os, pathlib, shutil, subprocess, json, re
ROOT=pathlib.Path(__file__).resolve().parents[1]
os.chdir(ROOT)
SDK=pathlib.Path(os.environ.get('WASI_SDK_PATH',ROOT/'tools/wasi-sdk-30.0-x86_64-linux')).resolve()
CLANG=str(SDK/'bin/clang'); GEN=ROOT/'build/generated'
BASE=[CLANG,'--target=wasm32-wasip2','-O3','-DNDEBUG','-D_WASI_EMULATED_SIGNAL','-D_WASI_EMULATED_MMAN','-mllvm','-wasm-enable-sjlj','-mllvm','-wasm-use-legacy-eh=false','-I','native/common','-I',str(GEN)]
C_EXPORTS=[f'-Wl,--export={x}' for x in sorted(set(re.findall(r'\b(wit_dylib_\w+)\(', (ROOT/'native/common/wit_dylib.h').read_text())) | {'wit_dylib_export_async_call','wit_dylib_export_async_callback','cabi_realloc'})]
LIBS=['-lsetjmp','-lwasi-emulated-signal','-lwasi-emulated-mman','-Wl,--strip-debug']
def run(args,env=None):
    print('+',' '.join(map(str,args)),flush=True)
    subprocess.run(list(map(str,args)),env=os.environ|dict(env or {}),check=True)
def rust(name,extra=(),env=None):
    run(['cargo','rustc','--locked','--release','--target','wasm32-wasip2','-p',name,'--','-Clink-self-contained=n',*extra],{'BINDINGS_DIR':str(GEN),'CARGO_TARGET_WASM32_WASIP2_LINKER':str(ROOT/'scripts/wasi-static-linker.py')}|dict(env or {}))
def link(engine):
    d=ROOT/'build'/f'{engine}-c';libdir=SDK/'share/wasi-sysroot/lib/wasm32-wasip2'
    libraries=['libc.so']+(['libsetjmp.so','libwasi-emulated-signal.so','libwasi-emulated-mman.so'] if engine=='porffor' else [])
    run(['wasm-tools','component','link',f'libengine.so={d}/libengine.so',f'world.so={GEN}/world.so',*[f'{x}={libdir}/{x}' for x in libraries],'--adapt','wasi_snapshot_preview1=upstream/componentize-js/adapters/3dda9169/wasi_snapshot_preview1.reactor.wasm','--use-built-in-libdl','--validate','-o',d/'component.wasm'])
def boa(approach):
    d=ROOT/'build'/f'boa-{approach}';d.mkdir(parents=True,exist_ok=True)
    if approach=='a':
        rust('boa-a',[f'-Clink-arg=-Wl,--component-type={ROOT}/wit/world.wit']); artifact=ROOT/'target/wasm32-wasip2/release/boa_a.wasm'
    elif approach=='b':
        for src,out in [('native/common/shims_b.c','shims.o'),('build/generated/bench.c','runtime.o')]:run([*BASE,'-c',src,'-o',d/out])
        rust('boa-native',[f'-Clink-arg={x}' for x in [d/'shims.o',d/'runtime.o',GEN/'bench_component_type.o']]);artifact=ROOT/'target/wasm32-wasip2/release/boa_native.wasm'
    else:
        for src,out in [('native/common/runtime_c.c','runtime.o'),('native/common/unsupported_c.c','unsupported.o')]:run([*BASE,'-fPIC','-c',src,'-o',d/out])
        rust('boa-native',[f'-Clink-arg={d/x}' for x in ['runtime.o','unsupported.o']]+[f'-Clink-arg={x}' for x in C_EXPORTS],{'CARGO_TARGET_DIR':str(ROOT/'target/pic'),'CARGO_TARGET_WASM32_WASIP2_LINKER':CLANG,'RUSTFLAGS':'-C relocation-model=pic -Clink-args=-Wl,--skip-wit-component -Clink-args=-shared -Clink-self-contained=n'})
        shutil.copyfile(ROOT/'target/pic/wasm32-wasip2/release/boa_native.wasm',d/'libengine.so');link('boa');return
    shutil.copyfile(artifact,d/'component.wasm')
def porffor(approach):
    d=ROOT/'build'/f'porffor-{approach}';d.mkdir(parents=True,exist_ok=True)
    if approach=='a':
        run(['target/release/generate','wit','--source','packages/app/app.js','--world','bench','--out-dir',d]); aliases=d/'esbuild-aliases.txt'
    else:
        run(['node','scripts/porffor-native.mjs',d]);aliases=GEN/'native-aliases.txt'
    run(['node_modules/.bin/esbuild',d/'main.js','--bundle','--format=esm','--platform=neutral',f'--outfile={d}/bundle.js','--log-level=warning',*aliases.read_text().splitlines()])
    run(['node','upstream/porffor/runtime/index.js','c','--module',d/'bundle.js','-o',d/'app.c'])
    if approach!='a':
        with (d/'app.c').open('a') as f:f.write((ROOT/'native/porffor/engine.c').read_text())
    if approach=='c':
        run([*BASE,'-fPIC','-shared','-Wl,--skip-wit-component',d/'app.c','native/common/runtime_c.c','native/common/unsupported_c.c','native/porffor/longjmp.S',*C_EXPORTS,'-Wl,--export=__c_longjmp',*LIBS,'-o',d/'libengine.so']);link('porffor')
    else:
        extra=[f'-Wl,--component-type={ROOT}/wit/world.wit'] if approach=='a' else ['native/common/shims_b.c',GEN/'bench.c',GEN/'bench_component_type.o']
        run([*BASE,'-mexec-model=reactor',d/'app.c',*extra,*LIBS,'-o',d/'component.wasm'])
def main():
    p=argparse.ArgumentParser();p.add_argument('cells',nargs='*',default=[f'{e}-{a}' for e in ['boa','porffor'] for a in 'abc']);p.add_argument('--skip-generate',action='store_true');args=p.parse_args()
    if not args.skip_generate:run(['cargo','run','--locked','--release','-p','generate'])
    for cell in args.cells:
        engine,approach=cell.split('-');assert engine in ('boa','porffor') and approach in 'abc'
        (boa if engine=='boa' else porffor)(approach)
        run(['wasm-tools','validate',f'build/{cell}/component.wasm'])
        run(['wasm-tools','strip','--all',f'build/{cell}/component.wasm','-o',f'build/{cell}/stripped.wasm'])
if __name__=='__main__':main()
