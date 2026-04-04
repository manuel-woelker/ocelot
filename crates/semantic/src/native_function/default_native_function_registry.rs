use super::assert_eq_native_function::AssertEqNativeFunction;
use super::assert_native_function::AssertNativeFunction;
use super::native_function_registry::NativeFunctionRegistry;
use super::println_native_function::PrintlnNativeFunction;

/// Creates the default compiler-provided native registry for the core module.
pub fn default_native_function_registry() -> NativeFunctionRegistry {
    let mut registry = NativeFunctionRegistry::new();
    registry.register("core::println", Box::new(PrintlnNativeFunction));
    registry.register("core::assert", Box::new(AssertNativeFunction));
    registry.register("core::assert_eq", Box::new(AssertEqNativeFunction));
    registry
}
