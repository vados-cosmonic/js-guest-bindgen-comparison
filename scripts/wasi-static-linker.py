#!/usr/bin/env python3
"""Rust's raw Wasm linker protocol, with WASI SDK 30 libc/reactor startup."""
import os,sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
sdk=Path(os.environ.get('WASI_SDK_PATH',root/'tools/wasi-sdk-30.0-x86_64-linux')).resolve()
lib=sdk/'share/wasi-sysroot/lib/wasm32-wasip2'
args=[];skip=False
for i,arg in enumerate(sys.argv[1:]):
    if skip:skip=False;continue
    if arg=='-flavor':skip=True;continue
    if arg=='-L' and i+2<len(sys.argv) and sys.argv[i+2].endswith('/self-contained'):
        skip=True;continue
    if arg.endswith('/crt1-reactor.o'):continue
    if arg.startswith('-Wl,'):args.extend(arg[4:].split(','))
    else:args.append(arg)
linker=str(sdk/'bin/wasm-component-ld')
os.execv(linker,[linker,'-L',str(lib),str(lib/'crt1-reactor.o'),*args])
