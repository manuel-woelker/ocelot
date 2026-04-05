use ocelot_base::file_path::FilePath;
use ocelot_base::result::{OcelotResult, ResultExt};
use ocelot_base::shared_string::SharedString;

pub fn module_name_from_path(
    execution_root: &FilePath,
    path: &FilePath,
) -> OcelotResult<SharedString> {
    let relative_path = path
        .as_path()
        .strip_prefix(execution_root.as_path())
        .with_context(|| {
            format!("internal error: `{path}` is not inside execution root `{execution_root}`")
        })?;
    let mut relative_path = relative_path.to_path_buf();
    relative_path.set_extension("");

    let segments = relative_path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    Ok(segments.join("::").into())
}
