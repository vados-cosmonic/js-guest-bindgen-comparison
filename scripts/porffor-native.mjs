import fs from 'node:fs';
import path from 'node:path';
const root=process.cwd(), out=path.resolve(process.argv[2]);fs.mkdirSync(out,{recursive:true});
const names=JSON.parse(fs.readFileSync('build/generated/exports.json'));
let js=`Porffor.c\`#include "engine.h"
static jsval engine_callbacks[7];
static inline double native_num_jv(jsval v) { return v.val; }
static inline double native_num_f(double v) { return v; }
#define NATIVE_NUM(x) _Generic((x), jsval: native_num_jv, default: native_num_f)(x)
static jsval native_import(size_t i, jsval args);
\`;
function _componentizeJsCallImport(i,args){let result=undefined;Porffor.c\`result=native_import((size_t)NATIVE_NUM(i),args);\`;return result;}
globalThis._componentizeJsCallImport = _componentizeJsCallImport;
// Ensure native builtin APIs used by the C shim are compiled into the engine.
const $nativeApiObject = { count:0, scale:0, label:'' };
const $nativeApiBytes = new Uint8Array(0);
const $nativeApiKey = 'count';
$nativeApiObject[$nativeApiKey];
`;
js+=fs.readFileSync('build/generated/globals.js','utf8')+'\n';
js+=`import * as app from '${root}/packages/app/app.js';\n`;
js+='function $registerNative() {\n';
for(let i=0;i<names.length;i++)js+=`let fn${i}=app.${names[i]};Porffor.c\`engine_callbacks[${i}]=fn${i};porf_gc_native_root_add(fn${i}.val,fn${i}.type);\`;\n`;
js+='}\n$registerNative();\n';
fs.writeFileSync(out+'/main.js',js);
