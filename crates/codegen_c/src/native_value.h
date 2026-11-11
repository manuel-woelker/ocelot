/* Ocelot native ABI v1: arguments borrow, string results own one reference. */
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
typedef struct {
    uint64_t tag; /**< Unit (0), boolean (1), or string (2). */
    uint64_t payload; /**< Boolean bits or an opaque OcString pointer. */
} OcValue;
/** Initializes report state; path is borrowed for the process lifetime, or NULL. */
void oc_init_v1(const char *);
/** Copies exact bytes, producing an owned string; NULL is allowed for zero length. */
OcValue oc_string(const unsigned char *, size_t);
/** Retains a borrowed handle and returns an owned reference. */
OcValue oc_keep(OcValue);
/** Releases one owned reference; primitives require no cleanup. */
void oc_release(OcValue);
/** Compares borrowed values by tag and contents, including embedded NUL bytes. */
int oc_equal(OcValue, OcValue);
/** Concatenates display representations into a new owned string. */
OcValue oc_concat(OcValue, OcValue);
/** Prints a borrowed string or boolean and returns unit. */
OcValue oc_print(OcValue);
/** Checks a boolean and reports the borrowed diagnostic on failure. */
OcValue oc_assert(OcValue, OcValue);
/** Checks borrowed values, reporting the diagnostic and expected/actual diff. */
OcValue oc_assert_eq(OcValue, OcValue, OcValue);
/** Writes a borrowed value's eval representation to the private report. */
void oc_eval(OcValue);
/** Writes success unless a scenario has already reported its final eval result. */
void oc_finish(void);
static inline OcValue oc_unit(void) { OcValue v = {0, 0}; return v; }
static inline OcValue oc_boolean(int b) { OcValue v = {1, b != 0}; return v; }
