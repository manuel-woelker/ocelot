use crate::runtime_value::RuntimeValue;
use crate::type_kind::TypeKind;
use ocelot_base::assertion_error::AssertionError;
use ocelot_base::error::OcelotError;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_pal::pal::Pal;
use std::fmt::Debug;

/// One native function implementation signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFunctionSignature {
    pub argument_types: Vec<TypeKind>,
}

impl NativeFunctionSignature {
    /// Creates one native signature from argument types.
    pub fn new(argument_types: Vec<TypeKind>) -> Self {
        Self { argument_types }
    }
}

/// Call-site context needed by native implementations.
pub struct NativeFunctionContext<'a> {
    pub pal: &'a dyn Pal,
    pub source_file: &'a SourceFile,
    pub expression_span: Span,
}

impl<'a> NativeFunctionContext<'a> {
    /// Creates one native function call context.
    pub fn new(pal: &'a dyn Pal, source_file: &'a SourceFile, expression_span: Span) -> Self {
        Self {
            pal,
            source_file,
            expression_span,
        }
    }
}

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

/// Creates one boxed native implementation from a fully qualified function name.
pub fn native_function_by_name(name: &str) -> Option<Box<dyn NativeFunction>> {
    match name {
        "core::println" => Some(Box::new(PrintlnNativeFunction)),
        "core::assert" => Some(Box::new(AssertNativeFunction)),
        "core::assert_eq" => Some(Box::new(AssertEqNativeFunction)),
        _ => None,
    }
}

/// Converts one native signature type into a user-facing type label.
pub fn native_type_label(type_kind: TypeKind) -> SharedString {
    match type_kind {
        TypeKind::Any => "any".into(),
        TypeKind::Boolean => "bool".into(),
        TypeKind::String => "string".into(),
        TypeKind::Unresolved => "unresolved".into(),
    }
}
