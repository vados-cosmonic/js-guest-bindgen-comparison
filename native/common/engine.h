#ifndef COMPARISON_ENGINE_H
#define COMPARISON_ENGINE_H
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
typedef uint32_t ev;
// Engine values are rooted until engine_reset(mark). No Canonical ABI here.
void engine_init(void);
uint32_t engine_mark(void);
void engine_reset(uint32_t mark);
ev engine_number(double x);
double engine_to_number(ev x);
ev engine_string(const uint8_t *ptr, size_t len);
size_t engine_to_string(ev x, uint8_t **ptr); // malloc-owned UTF-8 copy
// Both engines copy byte arrays. No engine aliases canonical linear memory.
ev engine_bytes(const uint8_t *ptr, size_t len);
size_t engine_to_bytes(ev x, uint8_t **ptr); // malloc-owned copy
ev engine_object(void);
void engine_set(ev object, const char *key, ev value);
ev engine_get(ev object, const char *key);
ev engine_call(size_t which, const ev *args, size_t count);
ev runtime_import(size_t which, const ev *args, size_t count);
#endif
