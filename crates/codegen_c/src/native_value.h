/* Ocelot native ABI v1: arguments borrow, string results own one reference. */
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
typedef struct { uint64_t tag, payload; } OcValue;
void oc_init_v1(const char *);
OcValue oc_string(const unsigned char *, size_t);
OcValue oc_keep(OcValue);
void oc_release(OcValue);
int oc_equal(OcValue, OcValue);
OcValue oc_concat(OcValue, OcValue);
OcValue oc_print(OcValue);
OcValue oc_assert(OcValue, OcValue);
OcValue oc_assert_eq(OcValue, OcValue, OcValue);
void oc_eval(OcValue);
void oc_finish(void);
static inline OcValue oc_unit(void) { OcValue v = {0, 0}; return v; }
static inline OcValue oc_boolean(int b) { OcValue v = {1, b != 0}; return v; }
