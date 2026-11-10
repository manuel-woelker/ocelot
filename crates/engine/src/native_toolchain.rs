use crate::native_build_request::NativeBuildRequest;
use ocelot_base::result::OcelotResult;
use ocelot_pal::pal::Pal;

/// Compiles and links emitted C without participating in language lowering.
pub trait NativeToolchain {
    /// Validates compiler availability before running a spec matrix.
    fn check(&self, pal: &dyn Pal) -> OcelotResult<()>;
    /// Produces the requested executable, reporting tool failures separately.
    fn build(&self, pal: &dyn Pal, request: &NativeBuildRequest) -> OcelotResult<()>;
}
