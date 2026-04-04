use ocelot_ast::type_kind::TypeKind;
use ocelot_base::assertion_error::AssertionError;
use ocelot_base::error::OcelotError;
use ocelot_base::result::OcelotResult;

use crate::runtime_value::RuntimeValue;

use super::native_function_context::NativeFunctionContext;
use super::native_function_signature::NativeFunctionSignature;
use super::native_function_trait::NativeFunction;

/// Native `assert` implementation.
#[derive(Debug, Clone, Default)]
pub struct AssertNativeFunction;

impl NativeFunction for AssertNativeFunction {
    fn apply(
        &self,
        arguments: &[RuntimeValue],
        context: &NativeFunctionContext<'_>,
    ) -> OcelotResult<RuntimeValue> {
        let condition =
            arguments[0].expect_boolean("type error: `assert` expects a bool argument")?;

        if condition {
            return Ok(RuntimeValue::unit());
        }

        Err(OcelotError::assertion_error(
            AssertionError::new_without_diff(
                context.source_file,
                context.expression_span.clone(),
                "assert condition was false",
            ),
        ))
    }

    fn signature(&self) -> NativeFunctionSignature {
        NativeFunctionSignature::new(vec![TypeKind::Boolean])
    }

    fn boxed_clone(&self) -> Box<dyn NativeFunction> {
        Box::new(self.clone())
    }
}
