/* Borrowed byte view; embedded NUL bytes are ordinary string contents. */
typedef struct {
    const unsigned char *data; /**< Borrowed data valid for the enclosing operation. */
    size_t length; /**< Number of readable bytes. */
} OcBytes;
