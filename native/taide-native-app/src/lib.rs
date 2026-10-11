#[path = "agent-hooks.rs"]
pub mod agent_hooks;
#[path = "app-file.rs"]
pub mod app_file;
#[path = "app-file-views.rs"]
mod app_file_views;
#[path = "app-file-write.rs"]
pub mod app_file_write;
pub mod application;
#[path = "application-ports.rs"]
pub mod application_ports;
pub mod bootstrap;
#[path = "breadcrumb-host.rs"]
mod breadcrumb_host;
#[path = "breadcrumb-menu.rs"]
mod breadcrumb_menu;
#[path = "breadcrumbs.rs"]
pub mod breadcrumbs;
#[cfg(test)]
#[path = "button-key-tests.rs"]
mod button_key_tests;
pub mod close_dialog;
#[path = "command-dispatch.rs"]
mod command_dispatch;
#[path = "command-palette.rs"]
mod command_palette;
#[path = "command-registry.rs"]
pub mod command_registry;
#[path = "css-motion.rs"]
#[cfg(test)]
mod css_motion;
pub mod delete_dialog;
pub mod diagnostics;
#[path = "editor-command-text.rs"]
mod editor_command_text;
#[path = "editor-completion.rs"]
mod editor_completion;
#[path = "editor-documentation.rs"]
mod editor_documentation;
#[path = "editor-documentation-code.rs"]
mod editor_documentation_code;
#[path = "editor-documentation-images.rs"]
mod editor_documentation_images;
#[path = "editor-folding.rs"]
mod editor_folding;
#[path = "editor-fonts.rs"]
mod editor_fonts;
#[path = "editor-formatting.rs"]
mod editor_formatting;
#[path = "editor-highlights.rs"]
mod editor_highlights;
#[path = "editor-indentation.rs"]
mod editor_indentation;
#[path = "editor-locations.rs"]
mod editor_locations;
#[path = "editor-markup.rs"]
mod editor_markup;
#[path = "editor-problems.rs"]
mod editor_problems;
#[path = "editor-rename.rs"]
mod editor_rename;
#[path = "editor-rename-controller.rs"]
mod editor_rename_controller;
pub mod editor_reveal;
#[path = "editor-symbols.rs"]
mod editor_symbols;
#[path = "editor-syntax.rs"]
pub mod editor_syntax;
#[path = "event-relay.rs"]
pub mod event_relay;
pub mod events;
pub mod explorer;
pub mod explorer_clipboard;
pub mod explorer_clipboard_owners;
pub mod explorer_delete;
pub mod explorer_move;
pub mod explorer_source_missing;
pub mod explorer_toolbar;
pub mod file_sync;
#[path = "font-preview.rs"]
mod font_preview;
pub mod host;
#[path = "ide-server.rs"]
pub mod ide_server;
#[path = "ide-tools.rs"]
pub mod ide_tools;
#[path = "keybinding-editor.rs"]
pub mod keybinding_editor;
#[path = "keybinding-search.rs"]
pub mod keybinding_search;
pub mod keymap;
pub mod lsp;
#[path = "lsp-process.rs"]
pub mod lsp_process;
pub mod lsp_workspace;
pub mod lsp_workspace_worker;
pub mod missing_draft;
#[cfg(test)]
mod modal;
#[path = "motion-preference.rs"]
mod motion_preference;
#[path = "navigation-icons.rs"]
mod navigation_icons;
pub mod open_with;
#[path = "peek-models.rs"]
mod peek_models;
pub mod persistence;
pub mod presentation;
#[path = "presentation-refresh.rs"]
pub mod presentation_refresh;
pub mod preview;
mod preview_animation;
pub mod preview_hwp;
mod preview_hwp_cache;
pub mod preview_hwp_preflight;
pub mod preview_hwp_surface;
#[cfg(target_os = "macos")]
mod preview_macos;
pub mod preview_pdf;
#[cfg(target_os = "macos")]
mod preview_pdf_macos;
pub mod preview_pdf_surface;
pub mod preview_presentation;
pub mod preview_presentation_cache;
pub mod preview_presentation_surface;
pub mod preview_spreadsheet;
pub mod preview_spreadsheet_biff;
pub mod preview_spreadsheet_cache;
pub mod preview_spreadsheet_csv;
pub mod preview_spreadsheet_html;
pub mod preview_spreadsheet_surface;
pub mod preview_spreadsheet_xlml;
pub mod preview_spreadsheet_xls;
pub mod preview_status;
mod preview_svg;
pub mod preview_web;
pub mod preview_web_cache;
pub mod preview_web_client;
pub mod preview_web_document;
mod preview_web_file;
pub mod preview_web_helper;
pub mod preview_web_host;
pub mod preview_web_http;
mod preview_web_io;
pub mod preview_web_media;
mod preview_web_published;
pub mod preview_web_range;
pub mod preview_web_resource;
pub mod preview_web_view;
mod problems;
#[path = "problems-icons.rs"]
mod problems_icons;
pub mod projects;
#[path = "remote-agents.rs"]
pub mod remote_agents;
#[path = "remote-ai.rs"]
pub mod remote_ai;
#[path = "remote-assets.rs"]
pub mod remote_assets;
#[cfg(test)]
#[path = "remote-content-tests.rs"]
mod remote_content_tests;
#[path = "remote-dispatch.rs"]
pub mod remote_dispatch;
#[path = "remote-files.rs"]
pub mod remote_files;
#[path = "remote-gateway.rs"]
pub mod remote_gateway;
#[path = "remote-git.rs"]
pub mod remote_git;
#[path = "remote-http.rs"]
pub mod remote_http;
#[path = "remote-ide.rs"]
pub mod remote_ide;
#[path = "remote-layout.rs"]
pub mod remote_layout;
#[path = "remote-lsp.rs"]
pub mod remote_lsp;
#[path = "remote-plugins.rs"]
pub mod remote_plugins;
#[path = "remote-preferences.rs"]
pub mod remote_preferences;
#[path = "remote-projects.rs"]
pub mod remote_projects;
#[path = "remote-search.rs"]
pub mod remote_search;
#[path = "remote-serving.rs"]
pub mod remote_serving;
#[path = "remote-sync.rs"]
pub mod remote_sync;
#[path = "remote-terminal.rs"]
pub mod remote_terminal;
#[path = "remote-utilities.rs"]
pub mod remote_utilities;
#[path = "remote-ws.rs"]
pub mod remote_ws;
pub mod save;
#[path = "settings-controls.rs"]
pub mod settings_controls;
#[path = "settings-integrations.rs"]
pub mod settings_integrations;
#[path = "settings-view.rs"]
pub mod settings_view;
mod shell_keymap;
#[cfg(test)]
#[path = "snippet-host-tests.rs"]
mod snippet_host_tests;
#[path = "status-chord.rs"]
mod status_chord;
#[path = "status-editor.rs"]
mod status_editor;
#[path = "status-ide.rs"]
mod status_ide;
#[path = "symbol-location-host.rs"]
mod symbol_location_host;
#[path = "symbol-navigation.rs"]
mod symbol_navigation;
#[path = "symbol-outline.rs"]
mod symbol_outline;
#[path = "symbol-sidebar.rs"]
mod symbol_sidebar;
mod system_fonts;
#[path = "system-usage.rs"]
mod system_usage;
#[path = "system-usage-view.rs"]
mod system_usage_view;
mod tab_close_batch;
pub mod tabs;
pub mod terminal_dispatch;
pub mod terminal_environment;
pub mod terminal_file_links;
mod terminal_fonts;
pub mod terminal_frames;
pub mod terminal_host;
pub mod terminal_links;
mod terminal_ruler;
mod terminal_settings;
pub mod terminal_surface;
pub mod terminal_tabs;
pub mod terminal_writer;
#[path = "theme-color-picker.rs"]
pub mod theme_color_picker;
#[path = "theme-draft.rs"]
pub mod theme_draft;
#[path = "theme-edit.rs"]
pub mod theme_edit;
#[path = "theme-editor.rs"]
#[cfg(test)]
pub(crate) mod theme_editor;
#[path = "theme-editor-tokens.rs"]
#[cfg(test)]
mod theme_editor_tokens;
#[path = "theme-live-preview.rs"]
#[cfg(test)]
mod theme_live_preview;
mod toast;
mod tooltips;
#[path = "ui-fonts.rs"]
mod ui_fonts;
#[path = "keybinding-icons.rs"]
#[cfg(test)]
pub(crate) mod ui_icons;
pub mod untitled;
pub mod workspace_activity;
pub mod workspace_delete;
pub mod workspace_rename;
#[path = "workspace-symbol-host.rs"]
pub mod workspace_symbol_host;
#[path = "workspace-symbols.rs"]
pub mod workspace_symbols;
mod zen;
