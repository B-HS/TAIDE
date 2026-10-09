use std::sync::atomic::{AtomicU64, Ordering};

use taide_model::{
    app_event::AppEvent,
    error::AppResult,
    locale::ResolvedLocale,
    settings::Settings,
    theme::{ResolvedTheme, ThemeType},
};
use taide_runtime::{AppState, locale_actions, theme_actions};

#[derive(Default)]
pub(crate) struct Changes {
    revision: AtomicU64,
    themes: AtomicU64,
    settings: AtomicU64,
}

impl Changes {
    pub(crate) fn record(&self, event: &AppEvent) {
        if matches!(
            event,
            AppEvent::SettingsChanged { .. } | AppEvent::ThemeChanged { .. }
        ) {
            self.revision.fetch_add(1, Ordering::AcqRel);
        }
        if matches!(event, AppEvent::ThemeChanged { .. }) {
            self.themes.fetch_add(1, Ordering::AcqRel);
        }
        if matches!(event, AppEvent::SettingsChanged { .. }) {
            self.settings.fetch_add(1, Ordering::AcqRel);
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    pub(crate) fn theme_revision(&self) -> u64 {
        self.themes.load(Ordering::Acquire)
    }

    pub(crate) fn settings_revision(&self) -> u64 {
        self.settings.load(Ordering::Acquire)
    }
}

#[derive(Clone, PartialEq)]
pub struct Inputs {
    pub(crate) settings: Settings,
    system_theme: String,
    system_language: String,
}

impl Inputs {
    pub fn new(settings: Settings, system_theme: &str, system_language: &str) -> Self {
        Self {
            system_theme: if settings.follow_system_theme {
                system_theme.into()
            } else {
                String::new()
            },
            system_language: if settings.language == "system" {
                system_language.into()
            } else {
                String::new()
            },
            settings,
        }
    }

    pub(crate) fn load(&self, state: &AppState) -> Resolved {
        Resolved {
            theme: theme_actions::theme_get_for_settings(state, &self.settings, &self.system_theme),
            locale: locale_actions::locale_get_for_language(
                state,
                &self.settings.language,
                &self.system_language,
            ),
        }
    }
}

pub struct Resolved {
    pub(crate) theme: AppResult<ResolvedTheme>,
    pub(crate) locale: AppResult<ResolvedLocale>,
}

#[derive(Clone, PartialEq)]
pub struct Request {
    sequence: u64,
    revision: u64,
    pub(crate) inputs: Inputs,
}

pub(crate) struct Refresh {
    attempted: Option<Request>,
    pending: Option<Request>,
    sequence: u64,
}

impl Refresh {
    pub(crate) fn new(inputs: Inputs, revision: u64) -> Self {
        Self {
            attempted: Some(Request {
                sequence: 0,
                revision,
                inputs,
            }),
            pending: None,
            sequence: 1,
        }
    }

    pub(crate) fn next(&self, inputs: Inputs, revision: u64) -> Option<Request> {
        if self.pending.is_some()
            || self
                .attempted
                .as_ref()
                .is_some_and(|last| last.inputs == inputs && last.revision == revision)
        {
            return None;
        }
        Some(Request {
            sequence: self.sequence,
            revision,
            inputs,
        })
    }

    pub(crate) fn submitted(&mut self, request: Request) {
        self.sequence = self.sequence.wrapping_add(1);
        self.pending = Some(request);
    }

    pub(crate) fn finish(&mut self, request: &Request, inputs: &Inputs, revision: u64) -> bool {
        if self.pending.as_ref() != Some(request) {
            return false;
        }
        self.pending = None;
        self.attempted = Some(request.clone());
        request.revision == revision && request.inputs == *inputs
    }

    pub(crate) fn cancel(&mut self) {
        self.pending = None;
        self.attempted = None;
    }
}

pub(crate) struct Appearances {
    pub(crate) visuals: eframe::egui::Visuals,
    pub(crate) shell: taide_native_ui::shell::ShellColors,
    pub(crate) editor: taide_native_ui::editor_surface::EditorAppearance,
    pub(crate) editor_display: taide_native_ui::editor_display::EditorDisplayColors,
    pub(crate) editor_brackets: taide_native_ui::editor_brackets::EditorBracketColors,
    pub(crate) editor_sticky: taide_native_ui::editor_sticky_scroll::EditorStickyColors,
    pub(crate) editor_minimap: taide_native_ui::editor_minimap::EditorMinimapColors,
    pub(crate) find: taide_native_ui::editor_find_widget::FindAppearance,
    pub(crate) banner: taide_native_ui::conflict_banner::BannerAppearance,
    pub(crate) lsp_status: crate::lsp::status::Appearance,
    pub(crate) status_editor: crate::status_editor::Appearance,
    pub(crate) status_ide: crate::status_ide::Appearance,
    pub(crate) system_usage: crate::system_usage_view::Appearance,
    pub(crate) status_chord: crate::status_chord::Appearance,
    pub(crate) problems: crate::problems::Appearance,
    pub(crate) explorer: crate::explorer::Appearance,
    pub(crate) tooltip: crate::tooltips::Appearance,
    pub(crate) terminal: crate::terminal_surface::Appearance,
    pub(crate) keybindings: crate::keybinding_editor::Appearance,
    pub(crate) palette: crate::command_palette::Appearance,
    pub(crate) settings: crate::settings_view::Appearance,
    pub(crate) pdf: crate::preview_pdf_surface::Appearance,
    pub(crate) presentation: crate::preview_presentation_surface::Appearance,
    pub(crate) spreadsheet: crate::preview_spreadsheet_surface::Appearance,
    pub(crate) toast: ThemeType,
}

pub(crate) fn editor_presentation(
    settings: &Settings,
) -> taide_native_ui::editor_surface::EditorPresentation {
    let mut presentation = taide_native_ui::presentation::editor_presentation(settings);
    presentation.options.bold_family = Some(eframe::egui::FontFamily::Name(
        taide_native_ui::font_families::EDITOR_BOLD_FAMILY.into(),
    ));
    presentation
}

impl Appearances {
    pub(crate) fn new(theme: &ResolvedTheme, settings: &Settings) -> AppResult<Self> {
        use crate::presentation::{color, editor_appearance, shell_colors, visuals};

        let status = || -> AppResult<crate::preview_status::Appearance> {
            Ok(crate::preview_status::Appearance {
                background: color(theme, "editor.background")?,
                border: color(theme, "editor.widgetBorder")?,
                foreground: color(theme, "editor.foreground")?,
                muted: color(theme, "appSidebar.iconDefault")?,
            })
        };
        let mut terminal =
            crate::terminal_surface::Appearance::new(theme, settings.terminal_font_size)?;
        terminal.font.family = crate::terminal_fonts::family();
        let mut editor = editor_appearance(theme, settings)?;
        editor.font.family = crate::editor_fonts::family();
        Ok(Self {
            visuals: visuals(theme)?,
            shell: shell_colors(theme)?,
            editor,
            editor_display: taide_native_ui::presentation::editor_display_colors(theme)?,
            editor_brackets: taide_native_ui::presentation::editor_bracket_colors(theme)?,
            editor_sticky: taide_native_ui::presentation::editor_sticky_colors(theme)?,
            editor_minimap: taide_native_ui::presentation::editor_minimap_colors(theme)?,
            find: taide_native_ui::editor_find_widget::FindAppearance {
                background: color(theme, "editor.widgetBackground")?,
                border: color(theme, "editor.widgetBorder")?,
                foreground: color(theme, "editor.foreground")?,
                input_background: color(theme, "input.background")?,
                input_foreground: color(theme, "input.foreground")?,
                input_border: color(theme, "input.border")?,
                focus: color(theme, "app.focusBorder")?,
                active_background: color(theme, "button.primaryBackground")?,
                active_foreground: color(theme, "button.primaryForeground")?,
                active_border: color(theme, "app.focusBorder")?,
                hover: color(theme, "list.hoverBackground")?,
                error: color(theme, "statusIndicator.error")?,
                highlight: color(theme, "editor.findMatchHighlight")?,
                current_match: color(theme, "editor.findMatch")?,
                shadow: color(theme, "app.shadow")?,
                quick_input_background: color(theme, "modal.background")?,
                quick_input_foreground: color(theme, "app.foreground")?,
                placeholder: color(theme, "input.placeholder")?,
                error_background: color(theme, "panel.background")?,
                error_foreground: color(theme, "app.foreground")?,
                scope: match theme.theme_type {
                    ThemeType::Dark => taide_native_ui::presentation::parse_color(
                        "#3a3d4166",
                        "editor.findRangeHighlightBackground",
                    )?,
                    ThemeType::Light => taide_native_ui::presentation::parse_color(
                        "#b4b4b44d",
                        "editor.findRangeHighlightBackground",
                    )?,
                },
            },
            banner: taide_native_ui::presentation::banner_appearance(theme)?,
            terminal,
            lsp_status: crate::lsp::status::Appearance::new(theme)?,
            status_editor: crate::status_editor::Appearance::new(theme)?,
            status_ide: crate::status_ide::Appearance::new(theme)?,
            system_usage: crate::system_usage_view::Appearance::new(theme)?,
            status_chord: crate::status_chord::Appearance::new(theme)?,
            problems: crate::problems::Appearance::new(theme)?,
            explorer: crate::explorer::Appearance::new(theme)?,
            tooltip: crate::tooltips::Appearance::new(theme)?,
            keybindings: crate::keybinding_editor::Appearance::new(theme)?,
            palette: crate::command_palette::Appearance::new(theme)?,
            settings: crate::settings_view::Appearance::new(theme)?,
            pdf: crate::preview_pdf_surface::Appearance {
                background: color(theme, "editor.background")?,
                header: color(theme, "editor.widgetBackground")?,
                border: color(theme, "editor.widgetBorder")?,
                foreground: color(theme, "editor.foreground")?,
                muted: color(theme, "appSidebar.iconDefault")?,
            },
            presentation: crate::preview_presentation_surface::Appearance {
                status: status()?,
                border: color(theme, "app.border")?,
                selected: color(theme, "explorer.itemSelected")?,
                hover: color(theme, "explorer.itemHover")?,
                warning: color(theme, "statusIndicator.warning")?,
            },
            spreadsheet: crate::preview_spreadsheet_surface::Appearance {
                status: status()?,
                header: color(theme, "editor.widgetBackground")?,
                selected_background: color(theme, "tabBar.tabActiveBackground")?,
                selected_foreground: color(theme, "tabBar.tabActiveForeground")?,
                inactive_foreground: color(theme, "tabBar.tabInactiveForeground")?,
                hover: color(theme, "explorer.itemHover")?,
                cell_border: color(theme, "app.border")?,
                warning: color(theme, "statusIndicator.warning")?,
            },
            toast: theme.theme_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use taide_model::{ids::ProjectId, paths::AppPaths};
    use taide_runtime::{EventSink, TaskSupervisor};
    use tokio::sync::Notify;

    use super::*;
    use crate::host::{HostBridge, HostCommand, HostReply};

    const DEADLINE: Duration = Duration::from_secs(3);

    #[test]
    fn native_presentation_theme_revision은_테마목록과_일반설정변경을_구분한다() {
        let changes = Changes::default();
        changes.record(&AppEvent::SettingsChanged {
            settings: Box::default(),
        });
        assert_eq!(changes.revision(), 1);
        assert_eq!(changes.theme_revision(), 0);
        assert_eq!(changes.settings_revision(), 1);
        changes.record(&AppEvent::ThemeChanged {
            theme_id: "taide-light".into(),
        });
        assert_eq!(changes.revision(), 2);
        assert_eq!(changes.theme_revision(), 1);
        assert_eq!(changes.settings_revision(), 1);
        changes.record(&AppEvent::TerminalExited {
            session_id: "synthetic".into(),
            code: None,
        });
        assert_eq!(changes.revision(), 2);
        assert_eq!(changes.theme_revision(), 1);
        assert_eq!(changes.settings_revision(), 1);
    }

    #[test]
    fn native_presentation_refresh는_snapshot_host와_최신요청수명을_보존한다() {
        struct Sink;
        impl EventSink for Sink {
            fn publish(&self, _: AppEvent) {}
        }
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-presentation-refresh-{}", ProjectId::new())),
        ));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let tasks = TaskSupervisor::new(runtime.handle().clone());
        let services = crate::bootstrap::services(state.clone(), tasks.clone(), Arc::new(Sink));
        let ready = Arc::new(Notify::new());
        let wake = ready.clone();
        let mut host = HostBridge::connect_with_clipboard_ports(
            services,
            Arc::new(move || wake.notify_one()),
            Arc::new(|_| panic!("unexpected clipboard write")),
            Arc::new(|| panic!("unexpected clipboard read")),
            None,
        )
        .unwrap();
        let changes = Changes::default();
        let initial = Inputs::new(state.settings.read().clone(), "dark", "en-US");
        let mut refresh = Refresh::new(initial.clone(), changes.revision());
        assert!(refresh.next(initial.clone(), changes.revision()).is_none());
        changes.record(&AppEvent::TerminalExited {
            session_id: "synthetic".into(),
            code: None,
        });
        assert_eq!(changes.revision(), 0);
        let requested = Settings {
            theme_id: "vscode-light-modern".into(),
            language: "ko".into(),
            ..initial.settings.clone()
        };
        changes.record(&AppEvent::SettingsChanged {
            settings: Box::new(requested.clone()),
        });
        let requested = Inputs::new(requested, "dark", "en-US");
        let request = refresh.next(requested.clone(), changes.revision()).unwrap();
        refresh.submitted(request.clone());
        assert!(refresh.next(initial.clone(), changes.revision()).is_none());
        *state.settings.write() = Settings {
            language: "ja".into(),
            ..initial.settings.clone()
        };
        changes.record(&AppEvent::SettingsChanged {
            settings: Box::new(state.settings.read().clone()),
        });
        host.submit(HostCommand::RefreshPresentation(Box::new(request)))
            .unwrap();
        runtime.block_on(async {
            let next_reply = |host: &mut HostBridge| host.poll();
            let first = tokio::time::timeout(DEADLINE, async {
                loop {
                    if let Some(reply) = next_reply(&mut host) {
                        break reply;
                    }
                    ready.notified().await;
                }
            })
            .await
            .unwrap();
            let HostReply::Presentation {
                request: first_request,
                result,
            } = first
            else {
                panic!("wrong reply")
            };
            let resolved = result.unwrap();
            assert_eq!(resolved.theme.unwrap().theme_type, ThemeType::Light);
            assert_eq!(resolved.locale.unwrap().id, "ko");
            let current = Inputs::new(state.settings.read().clone(), "dark", "en-US");
            assert!(!refresh.finish(&first_request, &current, changes.revision()));
            let latest = refresh.next(current.clone(), changes.revision()).unwrap();
            refresh.submitted(latest.clone());
            assert!(!refresh.finish(&first_request, &current, changes.revision()));
            assert!(refresh.next(current.clone(), changes.revision()).is_none());
            host.submit(HostCommand::RefreshPresentation(Box::new(latest)))
                .unwrap();
            let reply = tokio::time::timeout(DEADLINE, async {
                loop {
                    if let Some(reply) = host.poll() {
                        break reply;
                    }
                    ready.notified().await;
                }
            })
            .await
            .unwrap();
            let HostReply::Presentation { request, result } = reply else {
                panic!("wrong reply")
            };
            let resolved = result.unwrap();
            assert_eq!(resolved.theme.unwrap().theme_type, ThemeType::Dark);
            assert_eq!(resolved.locale.unwrap().id, "ja");
            assert!(refresh.finish(&request, &current, changes.revision()));
            assert!(refresh.next(current.clone(), changes.revision()).is_none());
            changes.record(&AppEvent::ThemeChanged {
                theme_id: current.settings.theme_id.clone(),
            });
            let request = refresh.next(current.clone(), changes.revision()).unwrap();
            refresh.submitted(request.clone());
            refresh.cancel();
            let replacement = refresh.next(current.clone(), changes.revision()).unwrap();
            assert_ne!(replacement.sequence, request.sequence);
            refresh.submitted(replacement.clone());
            assert!(!refresh.finish(&request, &current, changes.revision()));
            assert!(refresh.finish(&replacement, &current, changes.revision()));
            let invalid = Inputs::new(
                Settings {
                    theme_id: "../outside".into(),
                    ..current.settings
                },
                "dark",
                "en-US",
            );
            let request = refresh.next(invalid.clone(), changes.revision()).unwrap();
            refresh.submitted(request.clone());
            host.submit(HostCommand::RefreshPresentation(Box::new(request)))
                .unwrap();
            let reply = tokio::time::timeout(DEADLINE, async {
                loop {
                    if let Some(reply) = host.poll() {
                        break reply;
                    }
                    ready.notified().await;
                }
            })
            .await
            .unwrap();
            let HostReply::Presentation { request, result } = reply else {
                panic!("wrong reply")
            };
            let resolved = result.unwrap();
            assert!(resolved.theme.is_err());
            assert_eq!(resolved.locale.unwrap().id, "ja");
            assert!(refresh.finish(&request, &invalid, changes.revision()));
            assert!(refresh.next(invalid, changes.revision()).is_none());
            tokio::time::timeout(DEADLINE, host.disconnect())
                .await
                .unwrap()
                .unwrap();
        });
    }

    #[test]
    fn native_presentation_refresh는_정본_appearance와_설정시스템경계를_보존한다() {
        let state = AppState::new(AppPaths::new(std::env::temp_dir().join(format!(
            "taide-presentation-appearances-{}",
            ProjectId::new()
        ))));
        let settings = Settings {
            language: "en".into(),
            ..state.settings.read().clone()
        };
        *state.settings.write() = settings.clone();
        let first = Inputs::new(settings.clone(), "dark", "en-US");
        assert!(first == Inputs::new(settings.clone(), "light", "ja-JP"));
        for summary in theme_actions::theme_list(&state)
            .unwrap()
            .into_iter()
            .filter(|theme| theme.builtin)
        {
            let theme = theme_actions::theme_get(&state, summary.id).unwrap();
            let prepared = Appearances::new(&theme, &settings).unwrap();
            assert_eq!(prepared.toast, theme.theme_type);
            assert_eq!(
                prepared.pdf.background,
                crate::presentation::color(&theme, "editor.background").unwrap()
            );
            assert_eq!(
                prepared.shell.background,
                crate::presentation::color(&theme, "app.background").unwrap()
            );
            assert_eq!(prepared.visuals.panel_fill, prepared.shell.background);
            assert_eq!(
                prepared.visuals.dark_mode,
                theme.theme_type == ThemeType::Dark
            );
            assert_eq!(prepared.editor.font.size, settings.editor_font_size as f32);
            assert_eq!(prepared.editor.font.family, crate::editor_fonts::family());
            assert_eq!(
                prepared.editor_display.whitespace,
                crate::presentation::color(&theme, "editor.whitespace").unwrap()
            );
            assert_eq!(
                prepared.editor_display.scrollbar,
                crate::presentation::color(&theme, "scrollbar.thumb").unwrap()
            );
            assert_eq!(
                prepared.editor_display.scrollbar_hover,
                crate::presentation::color(&theme, "scrollbar.thumbHover").unwrap()
            );
            assert_eq!(
                prepared.editor_brackets,
                taide_native_ui::presentation::editor_bracket_colors(&theme).unwrap()
            );
            assert_eq!(
                prepared.editor_sticky,
                taide_native_ui::presentation::editor_sticky_colors(&theme).unwrap()
            );
            assert_eq!(
                prepared.editor_minimap,
                taide_native_ui::presentation::editor_minimap_colors(&theme).unwrap()
            );
            assert_eq!(
                prepared.terminal.font.family,
                crate::terminal_fonts::family()
            );
            let mut broken = theme;
            broken.colors.remove("editor.foreground");
            assert!(Appearances::new(&broken, &settings).is_err());
        }
        let following = Settings {
            follow_system_theme: true,
            language: "system".into(),
            ..settings.clone()
        };
        let light = Inputs::new(following.clone(), "light", "ko-KR");
        let dark = Inputs::new(following, "dark", "ja-JP");
        assert!(light != dark);
        let light = light.load(&state);
        let dark = dark.load(&state);
        assert_eq!(light.theme.unwrap().theme_type, ThemeType::Light);
        assert_eq!(dark.theme.unwrap().theme_type, ThemeType::Dark);
        assert_eq!(light.locale.unwrap().id, "ko");
        assert_eq!(dark.locale.unwrap().id, "ja");
        assert_eq!(state.settings.read().clone(), settings);
    }

    #[test]
    fn native_editor_presentation은_설정의_word_wrap과_편집기_굵은_글꼴_패밀리를_전달한다() {
        let mut settings = Settings::default();
        for word_wrap in [false, true] {
            settings.editor_word_wrap = word_wrap;
            let presentation = editor_presentation(&settings);
            assert_eq!(presentation.options.word_wrap, word_wrap);
            assert_eq!(
                presentation.options.bold_family,
                Some(eframe::egui::FontFamily::Name(
                    taide_native_ui::font_families::EDITOR_BOLD_FAMILY.into()
                ))
            );
        }
    }
}
