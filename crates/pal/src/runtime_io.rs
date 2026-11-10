//! Small synchronous I/O boundary for standalone native programs.
use ocelot_base::result::OcelotResult;
use std::io::Write;

/// Writes and flushes a native program's stdout.
pub fn stdout(text: &str) -> OcelotResult<()> {
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())?;
    out.flush()?;
    Ok(())
}

/// Writes native diagnostics without constructing a process execution runtime.
pub fn stderr(text: &str) -> OcelotResult<()> {
    std::io::stderr().lock().write_all(text.as_bytes())?;
    Ok(())
}

/// Writes a private native test report.
pub fn report(path: &std::path::Path, text: &str) -> OcelotResult<()> {
    std::fs::write(path, text)?;
    Ok(())
}

/// Terminates a standalone program after a reported failure.
pub fn exit(code: i32) -> ! {
    std::process::exit(code)
}
