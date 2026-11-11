/* Direct C ABI ownership and byte-semantics regression, also run under sanitizers. */
#include <assert.h>
int main(int argc, char **argv) {
    const unsigned char bytes[] = {0xc3, 0xa9, 0, 'x'};
    const unsigned char other[] = {0xc3, 0xa9, 0, 'y'};
    OcValue original, alias, same, different, empty, empty_copy, joined, expected;
    if (argc == 2 && !strcmp(argv[1], "--overflow")) {
        (void)oc_string(bytes, SIZE_MAX);
        return 99;
    }
    oc_init_v1(argc == 3 ? argv[2] : NULL);
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
