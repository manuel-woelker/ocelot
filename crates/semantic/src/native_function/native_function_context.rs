use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_pal::pal::Pal;

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
