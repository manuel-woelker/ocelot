use std::fmt::Debug;

use ocelot_base::result::OcelotResult;

use crate::runtime_value::RuntimeValue;

use super::native_function_context::NativeFunctionContext;
use super::native_function_signature::NativeFunctionSignature;

/// Runtime dispatch interface for one native function implementation.
pub trait NativeFunction: Debug + Send + Sync {
    /// Executes the native implementation for one evaluated argument list.
    fn apply(
        &self,
        arguments: &[RuntimeValue],
        context: &NativeFunctionContext<'_>,
    ) -> OcelotResult<RuntimeValue>;

    /// Returns the native function signature used for validation.
    fn signature(&self) -> NativeFunctionSignature;

    /// Clones the implementation behind a trait object.
    fn boxed_clone(&self) -> Box<dyn NativeFunction>;
}

impl Clone for Box<dyn NativeFunction> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}
