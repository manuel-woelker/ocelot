/* Direct C ABI ownership and byte-semantics regression, also run under sanitizers. */
#include <assert.h>
int main(int argc, char **argv) {
    const unsigned char bytes[] = {0xc3, 0xa9, 0, 'x'};
    const unsigned char other[] = {0xc3, 0xa9, 0, 'y'};
    OcValue original, alias, same, different, empty, empty_copy, joined, expected;
    int64_t integers[] = {INT64_MIN, INT64_MIN + 1, -3037000500LL, -2, -1, 0,
                          1, 2, 3037000500LL, INT64_MAX - 1, INT64_MAX};
    size_t i, j;
    if (argc == 2 && !strncmp(argv[1], "--int-", 6)) {
        OcValue diagnostic = oc_string((const unsigned char *)"checked integer error", 21);
        const char *mode = argv[1] + 6;
        if (!strcmp(mode, "negate")) (void)oc_int_negate(oc_integer(INT64_MIN), diagnostic);
        else {
            int64_t a = INT64_MAX, b = 1;
            OcIntegerOperator op = OC_ADD;
            if (!strcmp(mode, "subtract")) { a = INT64_MIN; op = OC_SUBTRACT; }
            if (!strcmp(mode, "multiply")) { a = INT64_MIN; b = -1; op = OC_MULTIPLY; }
            if (!strcmp(mode, "divide")) { a = INT64_MIN; b = -1; op = OC_DIVIDE; }
            if (!strcmp(mode, "zero")) { b = 0; op = OC_DIVIDE; }
            (void)oc_int_binary(oc_integer(a), oc_integer(b), op, diagnostic, diagnostic);
        }
        return 99;
    }
    if (argc == 2 && !strcmp(argv[1], "--overflow")) {
        (void)oc_string(bytes, SIZE_MAX);
        return 99;
    }
    oc_init_v1(argc == 3 ? argv[2] : NULL);
    for (i = 0; i < sizeof(integers) / sizeof(integers[0]); ++i) {
        int64_t a = integers[i];
        assert(oc_integer_value(oc_integer(a)) == a);
        if (a != INT64_MIN) assert(oc_integer_value(oc_int_negate(oc_integer(a), oc_unit())) == -a);
        for (j = 0; j < sizeof(integers) / sizeof(integers[0]); ++j) {
            int64_t b = integers[j];
#ifdef OCELOT_TEST_OVERFLOW_BUILTINS
            int64_t sum, difference, product;
            /* Independent compiler builtins are the oracle, never the runtime guards. */
            if (!__builtin_add_overflow(a, b, &sum))
                assert(oc_integer_value(oc_int_binary(oc_integer(a), oc_integer(b), OC_ADD, oc_unit(), oc_unit())) == sum);
            if (!__builtin_sub_overflow(a, b, &difference))
                assert(oc_integer_value(oc_int_binary(oc_integer(a), oc_integer(b), OC_SUBTRACT, oc_unit(), oc_unit())) == difference);
            if (!__builtin_mul_overflow(a, b, &product))
                assert(oc_integer_value(oc_int_binary(oc_integer(a), oc_integer(b), OC_MULTIPLY, oc_unit(), oc_unit())) == product);
#endif
            if (b && !(a == INT64_MIN && b == -1))
                assert(oc_integer_value(oc_int_binary(oc_integer(a), oc_integer(b), OC_DIVIDE, oc_unit(), oc_unit())) == a / b);
        }
    }
    joined = oc_concat(oc_integer(INT64_MIN), oc_integer(INT64_MAX));
    expected = oc_string((const unsigned char *)"-92233720368547758089223372036854775807", 39);
    assert(oc_equal(joined, expected));
    oc_release(joined);
    oc_release(expected);
    original = oc_string(bytes, sizeof(bytes));
    alias = oc_keep(original);
    oc_release(original);
    same = oc_string(bytes, sizeof(bytes));
    different = oc_string(other, sizeof(other));
    assert(oc_equal(alias, same));
    assert(!oc_equal(alias, different));
    assert(!oc_equal(alias, oc_unit()));
    empty = oc_string(NULL, 0);
    empty_copy = oc_string(NULL, 0);
    assert(oc_equal(empty, empty_copy));
    joined = oc_concat(oc_boolean(0), oc_unit());
    expected = oc_string((const unsigned char *)"false()", 7);
    assert(oc_equal(joined, expected));
    oc_eval(alias);
    oc_release(alias);
    oc_release(same);
    oc_release(different);
    oc_release(empty);
    oc_release(empty_copy);
    oc_release(joined);
    oc_release(expected);
    oc_finish();
    return 0;
}
