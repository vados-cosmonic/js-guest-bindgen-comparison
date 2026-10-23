#include <stdint.h>
#include <stdlib.h>
#include <string.h>

static jsval porf_call_dynamic(jsval fn, jsval thisv, jsval newtv, i32 argc, jsbits* argv);
int main(int argc, char** argv);

// JS locals have inferred C types (f64, i32 or jsval), so convert by type
static inline f64 wit_num_jv(jsval v) { return v.val; }
static inline f64 wit_num_f(f64 d) { return d; }
static inline jsval wit_jv_id(jsval v) { return v; }
static inline u32 wit_ptr_jv(jsval v) { return (u32)v.val; }
static inline u32 wit_ptr_u(u32 p) { return p; }
#define WIT_NUM(x) _Generic((x), jsval: wit_num_jv, default: wit_num_f)(x)
#define WIT_JV(x) _Generic((x), jsval: wit_jv_id, default: porf_box_num)(x)
#define WIT_PTR(x) _Generic((x), jsval: wit_ptr_jv, default: wit_ptr_u)(x)
#define WIT_SET(r, v) ((r) = _Generic((r), jsval: porf_box_num, default: wit_num_f)(v))
#define WIT_I32(x) ((int32_t)(int64_t)WIT_NUM(x))
#define WIT_U32(x) ((uint32_t)(int64_t)WIT_NUM(x))
#define WIT_ARG(v) porf_pack(porf_box_num((f64)(v)))
// absolute linear memory access, porffor's MEM is only part of it
#define WIT_LD(T, p) ({ T wit_v_; memcpy(&wit_v_, (void*)(uintptr_t)(p), sizeof(T)); wit_v_; })
#define WIT_ST(T, p, v) do { T wit_v_ = (T)(v); memcpy((void*)(uintptr_t)(p), &wit_v_, sizeof(T)); } while (0)
// first element of a porffor typed array (its data pointer is 4 bytes before)
#define WIT_TA_DATA(x) (MEM + *(u32*)(MEM + WIT_PTR(x) + 4) + 4)

// allocations made by JS lowering, freed once the host is done with them
static void** wit_allocs;
static uint32_t wit_nallocs, wit_capallocs;

static uint32_t wit_js_realloc(uint32_t size) {
  void* p = malloc(size ? size : 1);
  if (wit_nallocs == wit_capallocs) {
    wit_capallocs = wit_capallocs ? wit_capallocs * 2 : 16;
    wit_allocs = realloc(wit_allocs, wit_capallocs * sizeof(void*));
  }
  wit_allocs[wit_nallocs++] = p;
  return (uint32_t)(uintptr_t)p;
}

static void wit_free_tracked(void) {
  for (uint32_t i = 0; i < wit_nallocs; i++) free(wit_allocs[i]);
  wit_nallocs = 0;
}

// a reactor has no main call, so run the top level on first entry
static int wit_inited;
static void wit_init(void) {
  static char* argv[] = { "app", NULL };
  wit_inited = 1;
  main(1, argv);
}

#if PORF_GC_ENABLED
#define WIT_ENTER() if (!wit_inited) wit_init(); volatile int wit_anchor = 0; void* wit_prev_top = porf_c_stack_top; porf_c_stack_top = (void*)&wit_anchor
#define WIT_LEAVE() porf_c_stack_top = wit_prev_top
#else
#define WIT_ENTER() if (!wit_inited) wit_init()
#define WIT_LEAVE() (void)0
#endif

