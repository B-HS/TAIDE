pub(crate) use taide_infra::root_guard::canonicalize_lenient;
pub use taide_infra::root_guard::{
    ensure_existing_file, ensure_safe_component, ensure_within_root, project_root, resolve_entry_owning_project, resolve_owning_project,
    resolve_owning_project_or_cli_opened,
};
