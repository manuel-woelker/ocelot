//! High-level language pipeline orchestration for `ocelot`.

pub mod builtin_module;
pub mod core_module;
pub mod discovered_test;
pub mod engine;
pub mod engine_command;
pub mod engine_worker;
pub mod failed_test_result;
pub mod loaded_module;
pub mod loaded_program;
pub mod module_name_from_path;
pub mod source_file_kind;
pub mod test_run_summary;
