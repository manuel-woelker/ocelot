use ocelot_ast::type_kind::TypeKind;
use ocelot_base::result::OcelotResult;

use crate::runtime_value::RuntimeValue;

use super::native_function_context::NativeFunctionContext;
use super::native_function_signature::NativeFunctionSignature;
use super::native_function_trait::NativeFunction;

/// Native `println` implementation.
#[derive(Debug, Clone, Default)]
pub struct PrintlnNativeFunction;

impl NativeFunction for PrintlnNativeFunction {
    fn apply(
        &self,
        arguments: &[RuntimeValue],
        context: &NativeFunctionContext<'_>,
    ) -> OcelotResult<RuntimeValue> {
        let value = &arguments[0];
        let text = match value {
            RuntimeValue::Boolean(_) | RuntimeValue::String(_) => value.render_for_display(),
            RuntimeValue::Unit => {
                ocelot_base::bail!("type error: `println` expects a string or bool argument")
            }
        };

        context.pal.print(&format!("{text}\n"))?;
        Ok(RuntimeValue::unit())
    }

    fn signature(&self) -> NativeFunctionSignature {
        NativeFunctionSignature::new(vec![TypeKind::Any])
    }

    fn boxed_clone(&self) -> Box<dyn NativeFunction> {
        Box::new(self.clone())
    }
}
