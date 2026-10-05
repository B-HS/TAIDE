#[cfg(target_arch = "wasm32")]
mod browser;

#[path = "app-file-opens.rs"]
pub mod app_file_opens;
#[path = "app-files.rs"]
pub mod app_files;
pub mod close;
pub mod dirty;
pub mod files;
#[path = "mirror-writes.rs"]
pub mod mirror_writes;
pub mod mirrors;
pub mod preferences;
pub mod presentation;
#[path = "settings-catalog.rs"]
pub mod settings_catalog;
#[path = "settings-folders.rs"]
pub mod settings_folders;
#[path = "settings-resources.rs"]
pub mod settings_resources;
pub mod shell;
#[path = "snippet-operations.rs"]
pub mod snippet_operations;
#[path = "theme-operations.rs"]
pub mod theme_operations;

#[cfg(target_arch = "wasm32")]
#[path = "browser-shell.rs"]
mod browser_shell;

#[cfg(target_arch = "wasm32")]
pub use browser::{BrowserClient, BrowserError, BrowserEvent};

#[cfg(target_arch = "wasm32")]
pub use browser_shell::BrowserShell;

#[cfg(target_arch = "wasm32")]
#[path = "browser-workbench.rs"]
mod browser_workbench;

#[cfg(target_arch = "wasm32")]
pub use browser_workbench::{BrowserWorkbench, WorkbenchError};

#[cfg(target_arch = "wasm32")]
#[path = "browser-editor.rs"]
mod browser_editor;

#[cfg(target_arch = "wasm32")]
#[path = "font-preview.rs"]
mod font_preview;

#[cfg(target_arch = "wasm32")]
pub use browser_editor::{BrowserEditor, EditorBrowserError};

#[cfg(target_arch = "wasm32")]
#[path = "mirror-runtime.rs"]
mod mirror_runtime;

#[cfg(target_arch = "wasm32")]
#[path = "browser-application.rs"]
mod browser_application;

#[cfg(target_arch = "wasm32")]
pub use browser_application::{ApplicationError, BrowserApplication};

#[cfg(all(target_arch = "wasm32", feature = "canvas"))]
pub mod canvas;

pub use taide_remote_wire::client::{Delivery, InvokeError, ResponsePayload};
