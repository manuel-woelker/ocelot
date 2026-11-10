use ocelot_base::file_path::FilePath;

/// Toolchain-independent artifact paths for a native build.
pub struct NativeBuildRequest {
    /// Generated C translation unit.
    pub source: FilePath,
    /// Temporary object output.
    pub object: FilePath,
    /// Staged executable output.
    pub executable: FilePath,
    /// Matching ABI v1 static runtime archive.
    pub runtime: FilePath,
}
