// Interface-specific native engine shims. Typed WIT values enter/leave here;
// build/generated/bench.c separately implements Canonical ABI layout and calls.
#include "bench.h"
#include "engine.h"
#include <stdlib.h>
#include <assert.h>
static uint32_t begin(void){engine_init();return engine_mark();}
static ev invoke(size_t i,ev arg){return engine_call(i,&arg,1);}
static void string_result(ev v,bench_string_t *ret){ret->len=engine_to_string(v,&ret->ptr);if(!ret->len){free(ret->ptr);ret->ptr=NULL;}}
uint32_t exports_bench_scalar(uint32_t x){uint32_t m=begin();ev r=invoke(0,engine_number(x));uint32_t out=(uint32_t)engine_to_number(r);engine_reset(m);return out;}
double exports_bench_floating(double x){uint32_t m=begin();ev r=invoke(1,engine_number(x));double out=engine_to_number(r);engine_reset(m);return out;}
void exports_bench_text(bench_string_t *x,bench_string_t *out){uint32_t m=begin();ev a=engine_string(x->ptr,x->len);bench_string_free(x);string_result(invoke(2,a),out);engine_reset(m);}
void exports_bench_bytes(bench_list_u8_t *x,bench_list_u8_t *out){uint32_t m=begin();ev a=engine_bytes(x->ptr,x->len);bench_list_u8_free(x);out->len=engine_to_bytes(invoke(3,a),&out->ptr);if(!out->len){free(out->ptr);out->ptr=NULL;}engine_reset(m);}
void exports_bench_structured(bench_payload_t *x,bench_payload_t *out){
  uint32_t m=begin();ev a=engine_object();engine_set(a,"count",engine_number(x->count));engine_set(a,"scale",engine_number(x->scale));engine_set(a,"label",engine_string(x->label.ptr,x->label.len));bench_payload_free(x);
  ev r=invoke(4,a);out->count=(uint32_t)engine_to_number(engine_get(r,"count"));out->scale=engine_to_number(engine_get(r,"scale"));string_result(engine_get(r,"label"),&out->label);engine_reset(m);
}
uint32_t exports_bench_imports(uint32_t x){uint32_t m=begin();ev r=invoke(5,engine_number(x));uint32_t out=(uint32_t)engine_to_number(r);engine_reset(m);return out;}
void exports_bench_import_text(bench_string_t *x,bench_string_t *out){uint32_t m=begin();ev a=engine_string(x->ptr,x->len);bench_string_free(x);string_result(invoke(6,a),out);engine_reset(m);}
// Specialization of componentize-js's import entrypoint for this interface.
// The generated C runtime below these shims handles canonical argument layout.
ev runtime_import(size_t which,const ev *a,size_t n){
  assert(n==1);
  switch(which){
    case 0: return engine_number(comparison_bench_host_ping((uint32_t)engine_to_number(a[0])));
    case 1: {bench_string_t arg,out;string_result(a[0],&arg);comparison_bench_host_echo(&arg,&out);bench_string_free(&arg);ev result=engine_string(out.ptr,out.len);bench_string_free(&out);return result;}
    default: abort();
  }
}
