// Controlled diagnostics: remove only generated diagnostic statements; optional native UTF-8.
import fs from 'node:fs';
import parse from '../upstream/porffor/compiler/parser/index.js';
const [file,mode]=process.argv.slice(2);let src=fs.readFileSync(file,'utf8');
const spans=[];
function walk(n){if(!n||typeof n!=='object')return;if(n.type==='ExpressionStatement'&&n.expression.type==='CallExpression'&&n.expression.callee.type==='Identifier'&&n.expression.callee.name==='_debugLog'){spans.push([n.start,n.end]);return;}for(const v of Object.values(n)){if(Array.isArray(v))v.forEach(walk);else if(v&&typeof v==='object')walk(v);}}
walk(parse(src,{module:true}));if(spans.length!==18)console.log(`removed ${spans.length} diagnostic call statements`);
for(const [a,b] of spans.sort((a,b)=>b[0]-a[0]))src=src.slice(0,a)+src.slice(b);
if(mode==='native-utf8'){
 const start=src.indexOf('const TEXT_DECODER_UTF8 =');const end=src.indexOf('\nfunction $import_',start);
 if(start<0||end<0)throw new Error('prelude boundaries changed');
 src=src.slice(0,start)+`
const TEXT_DECODER_UTF8 = { decode(witDecodeBytes) {
  let witDecoded=undefined;
  Porffor.c\`witDecoded = wit_native_decode(WIT_JV(witDecodeBytes));\`;
  return witDecoded;
}};
function _utf8AllocateAndEncode(witEncodeString, realloc, memory) {
  if (typeof witEncodeString !== 'string') throw new TypeError('expected string');
  let witEncodedPtr=0, witEncodedLen=0;
  Porffor.c\`int32_t native_n;int32_t native_p=wit_native_encode(WIT_JV(witEncodeString),&native_n);WIT_SET(witEncodedPtr,(f64)native_p);WIT_SET(witEncodedLen,(f64)native_n);\`;
  return {ptr:witEncodedPtr,len:witEncodedLen};
}
`+src.slice(end);
 src=src.replace('static jsval porf_call_dynamic','static jsval wit_native_decode(jsval);\nstatic int32_t wit_native_encode(jsval,int32_t*);\nstatic jsval porf_call_dynamic');
}
fs.writeFileSync(file,src);
