// P2 port of componentize-js's wit-dylib Call/Interpreter contract.
// This layer has no Canonical ABI layout logic: actual wit-dylib owns that.
#include "engine.h"
#include "wit_dylib.h"
#include <stdlib.h>
#include <assert.h>
#include <string.h>
static const wit_t *world;
typedef struct { ev *stack; size_t len, cap; void **owned; size_t nowned, capowned; uint32_t mark; } Call;
static void push(Call *c, ev v) {
  if(c->len == c->cap) { c->cap = c->cap ? 2*c->cap : 16; c->stack = realloc(c->stack, c->cap*sizeof(ev)); }
  c->stack[c->len++] = v;
}
static ev pop(Call *c) { assert(c->len); return c->stack[--c->len]; }
static void own(Call *c, void *p) {
  if(c->nowned == c->capowned) { c->capowned = c->capowned ? 2*c->capowned : 8; c->owned = realloc(c->owned, c->capowned*sizeof(void*)); }
  c->owned[c->nowned++] = p;
}
static void finish(Call *c) {
  for(size_t i=0;i<c->nowned;i++) free(c->owned[i]);
  free(c->owned); free(c->stack);
}
void wit_dylib_initialize(const wit_t *wit) { assert(wit->version == WIT_CURRENT_VERSION); world = wit; }
void *wit_dylib_export_start(size_t which) {
  (void)which; engine_init(); Call *c = calloc(1,sizeof(Call)); c->mark=engine_mark(); return c;
}
void wit_dylib_export_call(void *cx, size_t which) {
  Call *c=cx; ev r=engine_call(which,c->stack,c->len); c->len=0;
  if(world->export_funcs[which].result != WIT_TYPE_EMPTY) push(c,r);
}
void wit_dylib_export_finish(void *cx,size_t which) {
  (void)which; Call *c=cx; engine_reset(c->mark); finish(c); free(c);
}
void wit_dylib_dealloc_bytes(void *cx,void *p,size_t size,size_t align,bool defer) {
  (void)size;(void)align; if(defer) own(cx,p); else free(p);
}
#define NUMBER(name,ctype) \
void wit_dylib_push_##name(void *cx,ctype v) { push(cx,engine_number(v)); } \
ctype wit_dylib_pop_##name(void *cx) { return (ctype)engine_to_number(pop(cx)); }
NUMBER(u8,uint8_t) NUMBER(u16,uint16_t) NUMBER(u32,uint32_t)
NUMBER(s8,int8_t) NUMBER(s16,int16_t) NUMBER(s32,int32_t)
NUMBER(f32,float) NUMBER(f64,double) NUMBER(bool,bool)
void wit_dylib_push_string(void *cx,uint8_t *p,size_t n) { push(cx,engine_string(p,n)); if(n) free(p); }
size_t wit_dylib_pop_string(void *cx,char **p) { size_t n=engine_to_string(pop(cx),(uint8_t**)p); own(cx,*p); return n; }
bool wit_dylib_push_list(void *cx,size_t ty,uint8_t *p,size_t n) {
  assert(world->lists[ty].ty == WIT_TYPE_U8); push(cx,engine_bytes(p,n)); if(n) free(p); return true;
}
size_t wit_dylib_pop_list(void *cx,size_t ty,void **p) {
  assert(world->lists[ty].ty == WIT_TYPE_U8); size_t n=engine_to_bytes(pop(cx),(uint8_t**)p);
  own(cx,*p); // Raw-list fast path borrows our buffer until finish.
  return n;
}
void wit_dylib_push_record(void *cx,size_t ty) {
  Call *c=cx; const wit_record_t *r=&world->records[ty]; ev o=engine_object();
  assert(c->len >= r->nfields); size_t start=c->len-r->nfields;
  for(size_t i=0;i<r->nfields;i++) engine_set(o,r->fields[i].name,c->stack[start+i]);
  c->len=start; push(c,o);
}
void wit_dylib_pop_record(void *cx,size_t ty) {
  Call *c=cx; const wit_record_t *r=&world->records[ty]; ev o=pop(c);
  for(size_t i=r->nfields;i>0;i--) push(c,engine_get(o,r->fields[i-1].name));
}
ev runtime_import(size_t which,const ev *args,size_t n) {
  assert(which<world->num_import_funcs); Call c={0};
  for(size_t i=n;i>0;i--) push(&c,args[i-1]);
  assert(world->import_funcs[which].impl); world->import_funcs[which].impl(&c);
  ev ret=c.len?pop(&c):engine_number(0); finish(&c); return ret;
}

// Supply the component allocator explicitly; libc.so can otherwise resolve its weak alias back to the linker trampoline.
__attribute__((weak)) void *cabi_realloc(void *p,size_t old,size_t align,size_t n){(void)old;if(!n)return (void*)(uintptr_t)align;void *q=realloc(p,n);if(!q)abort();return q;}
