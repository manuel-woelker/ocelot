/* Guard every signed operation before evaluation, including MIN / -1. */
/** Negates a borrowed integer, reporting the borrowed diagnostic on overflow. */
OcValue oc_int_negate(OcValue value, OcValue diagnostic) {
    int64_t operand = oc_integer_value(value);
    if (operand == INT64_MIN) oc_io_fail(oc_display(diagnostic, NULL));
    return oc_integer(-operand);
}

/** Evaluates checked arithmetic; diagnostic strings borrow their handles. */
OcValue oc_int_binary(OcValue left, OcValue right, OcIntegerOperator op,
                      OcValue overflow_diagnostic, OcValue zero_diagnostic) {
    int64_t a = oc_integer_value(left), b = oc_integer_value(right);
    int overflow = 0;
    switch (op) {
    case OC_ADD:
        overflow = (b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b);
        break;
    case OC_SUBTRACT:
        overflow = (b > 0 && a < INT64_MIN + b) || (b < 0 && a > INT64_MAX + b);
        break;
    case OC_MULTIPLY:
        if (a > 0) overflow = b > 0 ? a > INT64_MAX / b : b < INT64_MIN / a;
        else if (a < 0) overflow = b > 0 ? a < INT64_MIN / b : b < 0 && a < INT64_MAX / b;
        break;
    case OC_DIVIDE:
        if (!b) oc_io_fail(oc_display(zero_diagnostic, NULL));
        overflow = a == INT64_MIN && b == -1;
        break;
    }
    if (overflow) oc_io_fail(oc_display(overflow_diagnostic, NULL));
    switch (op) {
    case OC_ADD: return oc_integer(a + b);
    case OC_SUBTRACT: return oc_integer(a - b);
    case OC_MULTIPLY: return oc_integer(a * b);
    case OC_DIVIDE: return oc_integer(a / b);
    }
    oc_io_fatal("native runtime: invalid integer operator");
    return oc_unit();
}
