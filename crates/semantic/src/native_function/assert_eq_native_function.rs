use ocelot_ast::type_kind::TypeKind;
use ocelot_base::assertion_error::AssertionError;
use ocelot_base::error::OcelotError;
use ocelot_base::result::OcelotResult;

use crate::runtime_value::RuntimeValue;

use super::native_function_context::NativeFunctionContext;
use super::native_function_signature::NativeFunctionSignature;
use super::native_function_trait::NativeFunction;

/// Native `assert_eq` implementation.
#[derive(Debug, Clone, Default)]
pub struct AssertEqNativeFunction;

impl NativeFunction for AssertEqNativeFunction {
    fn apply(
        &self,
        arguments: &[RuntimeValue],
        context: &NativeFunctionContext<'_>,
    ) -> OcelotResult<RuntimeValue> {
        let expected = &arguments[0];
        let actual = &arguments[1];

        if expected.equals(actual) {
            return Ok(RuntimeValue::unit());
        }

        Err(OcelotError::assertion_error(AssertionError::new(
            context.source_file,
            context.expression_span.clone(),
            "assert_eq values differ",
            expected.render_for_assertion(),
            actual.render_for_assertion(),
        )))
    }

    fn signature(&self) -> NativeFunctionSignature {
        NativeFunctionSignature::new(vec![TypeKind::Any, TypeKind::Any])
    }

    fn boxed_clone(&self) -> Box<dyn NativeFunction> {
        Box::new(self.clone())
    }
}
