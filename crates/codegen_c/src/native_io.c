/* Synchronous platform boundary for native programs and private spec reports. */
#include <stdio.h>

static const char *oc_report_path;

static void oc_io_fatal(const char *message) {
    fputs(message, stderr);
    fputc('\n', stderr);
    exit(102);
}

static void oc_io_write(FILE *stream, const unsigned char *bytes, size_t length) {
    if (length && fwrite(bytes, 1, length, stream) != length)
        oc_io_fatal("native runtime: unable to write output");
}

static void oc_io_text(FILE *stream, const char *text) {
    oc_io_write(stream, (const unsigned char *)text, strlen(text));
}

static void oc_io_flush(FILE *stream) {
    if (fflush(stream)) oc_io_fatal("native runtime: unable to flush output");
}

static FILE *oc_io_report(const char *kind) {
    FILE *stream;
    if (!oc_report_path) return NULL;
    stream = fopen(oc_report_path, "wb");
    if (!stream) oc_io_fatal("native runtime: unable to open result report");
    oc_io_text(stream, "ocelot-");
    oc_io_text(stream, kind);
    oc_io_text(stream, "-v1\n");
    return stream;
}

static void oc_io_close_report(FILE *stream) {
    if (stream && fclose(stream))
        oc_io_fatal("native runtime: unable to close result report");
}

static void oc_io_fail(OcBytes message) {
    FILE *report = oc_io_report("error");
    if (report) oc_io_write(report, message.data, message.length);
    oc_io_close_report(report);
    oc_io_write(stderr, message.data, message.length);
    oc_io_text(stderr, "\n");
    oc_io_flush(stderr);
    exit(101);
}

static void oc_io_language_exit(void) { exit(101); }
