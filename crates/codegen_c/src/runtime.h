typedef struct { OcValue *values; size_t len, cap; } OcFrame;
static inline OcValue oc_track(OcFrame *f, OcValue v) {
    if (v.tag != 2) return v;
    if (f->len == f->cap) {
        size_t cap = f->cap ? f->cap * 2 : 16;
        OcValue *values = (OcValue *)realloc(f->values, cap * sizeof(OcValue));
        if (!values) abort();
        f->values = values; f->cap = cap;
    }
    f->values[f->len++] = v;
    return v;
}
static inline void oc_clear(OcFrame *f, size_t mark) {
    while (f->len > mark) oc_release(f->values[--f->len]);
}
static inline void oc_drop(OcFrame *f) { oc_clear(f, 0); free(f->values); }
