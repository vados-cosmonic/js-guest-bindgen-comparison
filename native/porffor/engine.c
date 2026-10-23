// Included after Porffor's generated C, so native engine builtins are visible.
#include "engine.h"
#include <assert.h>
static jsval *evs;
static i32 *roots;
static size_t evn, evcap;
static jsval engine_callbacks[7];
static int native_inited;
static ev keep(jsval v) {
  if(evn==evcap){evcap=evcap?evcap*2:32;evs=realloc(evs,evcap*sizeof(jsval));roots=realloc(roots,evcap*sizeof(i32));}
  evs[evn]=v; roots[evn]=porf_gc_native_root_add(v.val,v.type); return evn++;
}
void engine_init(void) {if(!native_inited){native_inited=1;static char *argv[]={"app",NULL};main(1,argv);}}
uint32_t engine_mark(void){return evn;}
void engine_reset(uint32_t m){while(evn>m){--evn;porf_gc_native_root_remove(roots[evn]);}}
ev engine_number(double v){return keep(porf_box_num(v));}
double engine_to_number(ev v){return porf_to_num(evs[v]);}
ev engine_string(const uint8_t *bytes,size_t n){
  bool ascii=true;for(size_t i=0;i<n;i++) if(bytes[i]&128){ascii=false;break;}
  u32 p=porf_alloc(4+n*(ascii?1:2),ascii?195:67);uint32_t len=0;
  if(ascii){memcpy(MEM+p+4,bytes,n);len=n;}
  else {uint16_t *out=(uint16_t*)(MEM+p+4);for(size_t i=0;i<n;){
    uint32_t c=bytes[i++];if(c>=240){c=((c&7)<<18)|((bytes[i]&63)<<12)|((bytes[i+1]&63)<<6)|(bytes[i+2]&63);i+=3;}
    else if(c>=224){c=((c&15)<<12)|((bytes[i]&63)<<6)|(bytes[i+1]&63);i+=2;}
    else if(c>=192){c=((c&31)<<6)|(bytes[i++]&63);}
    if(c>=0x10000){c-=0x10000;out[len++]=0xd800+(c>>10);out[len++]=0xdc00+(c&1023);}else out[len++]=c;
  }}
  *(u32*)(MEM+p)=len;return keep(porf_box(p,ascii?195:67));
}
size_t engine_to_string(ev x,uint8_t **out){
  jsval v=evs[x];assert(v.type==67||v.type==195);u32 p=(u32)v.val,n=*(u32*)(MEM+p);
  uint8_t *buf=malloc((size_t)n*3+1);size_t used=0;
  for(u32 i=0;i<n;i++){
    uint32_t c=v.type==195?MEM[p+4+i]:((uint16_t*)(MEM+p+4))[i];
    if(c>=0xd800&&c<=0xdbff&&i+1<n){uint32_t d=((uint16_t*)(MEM+p+4))[i+1];if(d>=0xdc00&&d<=0xdfff){c=0x10000+((c-0xd800)<<10)+(d-0xdc00);i++;}}
    if(c>=0xd800&&c<=0xdfff)c=0xfffd;
    if(c<128)buf[used++]=c;
    else if(c<2048){buf[used++]=192|(c>>6);buf[used++]=128|(c&63);}
    else if(c<65536){buf[used++]=224|(c>>12);buf[used++]=128|((c>>6)&63);buf[used++]=128|(c&63);}
    else{buf[used++]=240|(c>>18);buf[used++]=128|((c>>12)&63);buf[used++]=128|((c>>6)&63);buf[used++]=128|(c&63);}
  }*out=buf;return used;
}
ev engine_bytes(const uint8_t *p,size_t n){
  jsval v=p_builtins_Uint8Array(porf_box(p_builtins__23get_Uint8Array(),6),JV_UNDEFINED,porf_box_num(n),JV_UNDEFINED,JV_UNDEFINED);
  ev h=keep(v);memcpy(MEM+*(u32*)(MEM+(u32)v.val+4)+4,p,n);return h;
}
size_t engine_to_bytes(ev x,uint8_t **out){
  jsval v=evs[x];u32 n=*(u32*)(MEM+(u32)v.val);*out=malloc(n?n:1);
  memcpy(*out,MEM+*(u32*)(MEM+(u32)v.val+4)+4,n);return n;
}
ev engine_object(void){return keep(p_builtins___Porffor_object_new(4));}
void engine_set(ev o,const char *key,ev value){ev k=engine_string((const uint8_t*)key,strlen(key));p_builtins___Porffor_object_fastAdd(evs[o],evs[k],evs[value],14);}
ev engine_get(ev o,const char *key){ev k=engine_string((const uint8_t*)key,strlen(key));return keep(p_builtins___Porffor_object_get(evs[o],evs[k]));}
ev engine_call(size_t i,const ev *a,size_t n){
  jsbits args[n?n:1];for(size_t j=0;j<n;j++)args[j]=porf_pack(evs[a[j]]);
#if PORF_GC_ENABLED
  volatile int anchor=0;void *prev=porf_c_stack_top;porf_c_stack_top=(void*)&anchor;
#endif
  jsval v=porf_call_dynamic(engine_callbacks[i],JV_UNDEFINED,JV_UNDEFINED,n,args);
  ev r=keep(v);
#if PORF_GC_ENABLED
  porf_c_stack_top=prev;
#endif
  return r;
}
static jsval native_import(size_t i,jsval args){
  uint32_t mark=engine_mark();u32 n=*(u32*)(MEM+(u32)args.val);ev values[n?n:1];
  for(u32 j=0;j<n;j++)values[j]=keep(porf_arr_get((u32)args.val,j));
  ev ret=runtime_import(i,values,n);jsval result=evs[ret];engine_reset(mark);return result;
}
