/// Native functions currently supported by the interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeFunction {
    Assert,
    AssertEq,
    Println,
}
