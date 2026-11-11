/* ABI v1 implementation. Arguments borrow handles; string results own them. */
static int oc_evaluated;

static size_t oc_add_size(size_t left, size_t right) {
    if (right > SIZE_MAX - left) oc_io_fatal("native runtime: string size overflow");
    return left + right;
}

static OcString *oc_allocate_string(size_t length) {
    OcString *string = (OcString *)malloc(oc_add_size(sizeof(OcString), length));
    if (!string) oc_io_fatal("native runtime: allocation failed");
    string->references = 1;
    string->length = length;
    return string;
}

static OcValue oc_string_value(OcString *string) {
    OcValue value = {2, (uint64_t)(uintptr_t)string};
    return value;
}

static OcString *oc_storage(OcValue value) {
    return (OcString *)(uintptr_t)value.payload;
}

static OcBytes oc_display(OcValue value) {
    OcBytes bytes;
    if (value.tag == 2) {
        OcString *string = oc_storage(value);
        bytes.data = string->bytes;
        bytes.length = string->length;
    } else if (value.tag == 1) {
        bytes.data = (const unsigned char *)(value.payload ? "true" : "false");
        bytes.length = value.payload ? 4 : 5;
    } else {
        bytes.data = (const unsigned char *)"()";
        bytes.length = 2;
    }
    return bytes;
}

static void oc_write_value(FILE *stream, OcValue value, int quoted) {
    OcBytes bytes = oc_display(value);
    if (quoted && value.tag == 2) oc_io_text(stream, "\"");
    oc_io_write(stream, bytes.data, bytes.length);
    if (quoted && value.tag == 2) oc_io_text(stream, "\"");
}

void oc_init_v1(const char *path) {
    oc_report_path = path;
    oc_evaluated = 0;
}

OcValue oc_string(const unsigned char *bytes, size_t length) {
    OcString *string = oc_allocate_string(length);
    if (length) memcpy(string->bytes, bytes, length);
    return oc_string_value(string);
}

OcValue oc_keep(OcValue value) {
    if (value.tag == 2) {
        OcString *string = oc_storage(value);
        if (string->references == SIZE_MAX) oc_io_fatal("native runtime: reference count overflow");
        ++string->references;
    }
    return value;
}

void oc_release(OcValue value) {
    if (value.tag == 2) {
        OcString *string = oc_storage(value);
        if (!--string->references) free(string);
    }
}

int oc_equal(OcValue left, OcValue right) {
    OcBytes a, b;
    if (left.tag != right.tag) return 0;
    if (left.tag != 2) return left.payload == right.payload;
    a = oc_display(left);
    b = oc_display(right);
    return a.length == b.length && (!a.length || !memcmp(a.data, b.data, a.length));
}

OcValue oc_concat(OcValue left, OcValue right) {
    OcBytes a = oc_display(left), b = oc_display(right);
    OcString *string = oc_allocate_string(oc_add_size(a.length, b.length));
    if (a.length) memcpy(string->bytes, a.data, a.length);
    if (b.length) memcpy(string->bytes + a.length, b.data, b.length);
    return oc_string_value(string);
}

OcValue oc_print(OcValue value) {
    if (!value.tag) {
        const char *text = "type error: `println` expects a string or bool argument";
        OcBytes message = {(const unsigned char *)text, strlen(text)};
        oc_io_fail(message);
    }
    oc_write_value(stdout, value, 0);
    oc_io_text(stdout, "\n");
    oc_io_flush(stdout);
    return oc_unit();
}

OcValue oc_assert(OcValue condition, OcValue diagnostic) {
    if (!condition.payload) oc_io_fail(oc_display(diagnostic));
    return oc_unit();
}

static void oc_write_diff(FILE *stream, OcValue diagnostic, OcValue expected, OcValue actual) {
    oc_write_value(stream, diagnostic, 0);
    oc_io_text(stream, "\n\nexpected: ");
    oc_write_value(stream, expected, 1);
    oc_io_text(stream, "\nactual:   ");
    oc_write_value(stream, actual, 1);
}

OcValue oc_assert_eq(OcValue expected, OcValue actual, OcValue diagnostic) {
    if (!oc_equal(expected, actual)) {
        FILE *report = oc_io_report("error");
        if (report) oc_write_diff(report, diagnostic, expected, actual);
        oc_io_close_report(report);
        oc_write_diff(stderr, diagnostic, expected, actual);
        oc_io_text(stderr, "\n");
        oc_io_flush(stderr);
        oc_io_language_exit();
    }
    return oc_unit();
}

void oc_eval(OcValue value) {
    FILE *report = oc_io_report("eval");
    if (report) oc_write_value(report, value, 1);
    oc_io_close_report(report);
    oc_evaluated = 1;
}

void oc_finish(void) {
    if (!oc_evaluated) oc_io_close_report(oc_io_report("ok"));
}
