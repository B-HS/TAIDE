use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_infra::{lsp_install, lsp_proc};
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::lsp::{LanguageServerSpec, LspServerId};
use taide_model::paths::AppPaths;

fn managed_dir_for(paths: &AppPaths, server_id: &LspServerId) -> Option<PathBuf> {
    lsp_install::latest_installed_version(&paths.lsp_dir(), server_id.as_str())
        .map(|version| paths.lsp_server_version_dir(server_id.as_str(), &version))
}

fn resolved_args(spec: &LanguageServerSpec, root: &str, managed_dir: Option<&Path>) -> Vec<String> {
    let mut vars = vec![("workspaceDir", root.to_string())];
    if let Some(managed_dir) = managed_dir {
        vars.push(("serverDir", managed_dir.to_string_lossy().to_string()));
    }
    lsp_install::substitute_template_args(spec.command.args(), &vars)
}

pub fn resolve_process_config(
    paths: &AppPaths,
    spec: &LanguageServerSpec,
    root: &str,
    path_var: &OsStr,
) -> AppResult<lsp_proc::LspProcConfig> {
    let managed_dir = managed_dir_for(paths, &spec.id);
    let managed_relative_path = spec
        .install
        .download
        .as_ref()
        .and_then(|download| download.bin_path_in_archive.as_deref());
    let Some(resolved) = crate::service::resolve_spec_command(
        spec,
        Some(Path::new(root)),
        managed_dir.as_deref(),
        managed_relative_path,
        path_var,
    ) else {
        log::warn!(
            "lsp {}: executable not found (bin={}, root={root})",
            spec.id,
            spec.command.bin()
        );
        return Err(AppError::NotFound(format!(
            "language server executable not found: {}",
            spec.command.bin()
        )));
    };

    let args = resolved_args(spec, root, managed_dir.as_deref());
    if args
        .iter()
        .any(|arg| arg.contains('{') && arg.contains('}'))
    {
        return Err(AppError::localized(
            AppErrorKind::Internal,
            "error.lsp.unresolvedArgTemplate",
            format!("{}: unresolved template left in launch args (managed directory may not be installed)", spec.id),
        )
        .with_arg("serverId", &spec.id));
    }

    Ok(lsp_proc::LspProcConfig {
        command: resolved.to_string_lossy().to_string(),
        args,
        cwd: PathBuf::from(root),
    })
}

pub fn spawn_language_server<D, X>(
    paths: &AppPaths,
    spec: &LanguageServerSpec,
    root: &str,
    on_message: D,
    on_exit: X,
) -> AppResult<Arc<lsp_proc::LspProcHandle>>
where
    D: Fn(String) + Send + 'static,
    X: FnOnce(Option<i32>, String) + Send + 'static,
{
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let config = resolve_process_config(paths, spec, root, &path_var)?;
    let command = config.command.clone();
    let args = config.args.clone();
    let handle = lsp_proc::spawn(config, on_message, on_exit)?;

    log::info!(
        "lsp {}: spawn {command} {args:?} cwd={root} pid={:?}",
        spec.id,
        handle.pid()
    );

    Ok(Arc::new(handle))
}
