// Unreachable for the checked comparison world; fail loudly for unsupported types.
#include "wit_dylib.h"
#include <stdlib.h>
void wit_dylib_resource_dtor(size_t ty, size_t handle) { abort(); }
void wit_dylib_push_char(void *cx, uint32_t val) { abort(); }
void wit_dylib_push_u64(void *cx, uint64_t val) { abort(); }
void wit_dylib_push_s64(void *cx, int64_t val) { abort(); }
void wit_dylib_push_flags(void *cx, size_t ty, uint32_t flags) { abort(); }
void wit_dylib_push_enum(void *cx, size_t ty, uint32_t enum_) { abort(); }
void wit_dylib_push_borrow(void *cx, size_t ty, uint32_t handle) { abort(); }
void wit_dylib_push_own(void *cx, size_t ty, uint32_t handle) { abort(); }
void wit_dylib_push_future(void *cx, size_t ty, uint32_t handle) { abort(); }
void wit_dylib_push_stream(void *cx, size_t ty, uint32_t handle) { abort(); }
void wit_dylib_push_tuple(void *cx, size_t ty) { abort(); }
void wit_dylib_push_option(void *cx, size_t ty, uint32_t discr) { abort(); }
void wit_dylib_push_result(void *cx, size_t ty, uint32_t discr) { abort(); }
void wit_dylib_push_variant(void *cx, size_t ty, uint32_t discr) { abort(); }
void wit_dylib_list_append(void *cx, size_t ty) { abort(); }
uint64_t wit_dylib_pop_u64(void *cx) { abort(); }
int64_t wit_dylib_pop_s64(void *cx) { abort(); }
uint32_t wit_dylib_pop_char(void *cx) { abort(); }
uint32_t wit_dylib_pop_borrow(void *cx, size_t ty) { abort(); }
uint32_t wit_dylib_pop_own(void *cx, size_t ty) { abort(); }
uint32_t wit_dylib_pop_enum(void *cx, size_t ty) { abort(); }
uint32_t wit_dylib_pop_flags(void *cx, size_t ty) { abort(); }
uint32_t wit_dylib_pop_future(void *cx, size_t ty) { abort(); }
uint32_t wit_dylib_pop_stream(void *cx, size_t ty) { abort(); }
uint32_t wit_dylib_pop_option(void *cx, size_t type_index) { abort(); }
uint32_t wit_dylib_pop_result(void *cx, size_t type_index) { abort(); }
uint32_t wit_dylib_pop_variant(void *cx, size_t type_index) { abort(); }
void wit_dylib_pop_tuple(void *cx, size_t ty) { abort(); }
void wit_dylib_pop_iter_next(void *cx, size_t ty) { abort(); }
void wit_dylib_pop_iter(void *cx, size_t ty) { abort(); }
uint32_t wit_dylib_export_async_call(void *cx, size_t which) { abort(); }
uint32_t wit_dylib_export_async_callback(void *cx, uint32_t a,uint32_t b,uint32_t c) { abort(); }
