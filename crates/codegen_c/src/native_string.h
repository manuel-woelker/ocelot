/* Immutable, length-aware string storage; native execution is single-threaded. */
typedef struct {
    size_t references; /**< Number of owned handles, always positive while live. */
    size_t length; /**< Exact byte length, excluding any implicit terminator. */
    unsigned char bytes[]; /**< Immutable payload; no trailing NUL is required. */
} OcString;
