use crate::native_build_request::NativeBuildRequest;
use crate::native_toolchain::NativeToolchain;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::{OcelotResult, ResultExt};
use ocelot_pal::pal::Pal;
use std::sync::atomic::{AtomicU64, Ordering};

/// Locates the ABI v1 archive shipped beside the compiler or its test binaries.
pub fn runtime_archive() -> OcelotResult<FilePath> {
    let exe = std::env::current_exe()?;
    let mut directory = exe.parent().expect("executable parent").to_path_buf();
    if directory.file_name().is_some_and(|name| name == "deps") {
        directory.pop();
    }
    Ok(FilePath::new(directory.join("libocelot_native_runtime.a")))
}

/// Builds in an isolated sibling directory and publishes only a successful artifact.
pub fn build_native(
    pal: &dyn Pal,
    source: &str,
    output: &FilePath,
    runtime: &FilePath,
    toolchain: &dyn NativeToolchain,
) -> OcelotResult<()> {
    let output = pal.absolute_path(output)?;
    let runtime = pal.absolute_path(runtime)?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = output.parent().unwrap_or_default();
    pal.create_directory_all(&parent)?;
    let staging = parent.join(format!(
        ".ocelot-build-{}-{}-{}",
        std::process::id(),
        pal.now().as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    if !pal.create_directory(&staging)? {
        ocelot_base::bail!("native staging directory collision");
    }
    let request = NativeBuildRequest {
        source: staging.join("program.c"),
        object: staging.join("program.o"),
        executable: staging.join("program"),
        runtime,
    };
    pal.write_file(&request.source, source.as_bytes())?;
    toolchain
        .build(pal, &request)
        .with_context(|| format!("native build artifacts: {staging}"))?;
    pal.rename(&request.executable, &output)?;
    pal.remove_directory_all(&staging)?;
    Ok(())
}
