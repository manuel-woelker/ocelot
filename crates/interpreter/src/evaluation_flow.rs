use ocelot_semantic::runtime_value::RuntimeValue;
use std::ops::ControlFlow;

/// A normal expression value or a return from the enclosing function.
pub(crate) type EvaluationFlow = ControlFlow<RuntimeValue, RuntimeValue>;
