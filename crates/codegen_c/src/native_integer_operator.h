/** Checked integer operations; ordering uses ordinary signed C comparisons. */
typedef enum {
    OC_ADD, /**< Addition. */
    OC_SUBTRACT, /**< Subtraction. */
    OC_MULTIPLY, /**< Multiplication. */
    OC_DIVIDE /**< Division, truncating toward zero. */
} OcIntegerOperator;
