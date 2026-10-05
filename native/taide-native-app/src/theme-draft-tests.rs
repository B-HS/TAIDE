use super::*;
use taide_model::{
    error::AppErrorKind,
    ids::ProjectId,
    paths::AppPaths,
    theme::{TokenColorRule, TokenColorSettings},
};

#[test]
fn native_theme_draft는_import_메타데이터와_diff_reset_dirty를_보존한다() {
    let directory =
        std::env::temp_dir().join(format!("taide-native-theme-draft-{}", ProjectId::new()));
    std::fs::create_dir_all(&directory).unwrap();
    let state = AppState::new(AppPaths::new(directory.clone()));
    let duplicate = Draft::load(&state, "taide-dark", Mode::Create, "복사본".into()).unwrap();
    assert!(!duplicate.has_unsaved_changes());
    assert_eq!(duplicate.changed_count(), 0);
    let mut imported = duplicate.build().unwrap();
    imported.id = "synthetic-import".into();
    imported.name = "Synthetic Import".into();
    imported
        .colors
        .insert("app.accent".into(), "#12345678".into());
    imported.syntax.insert(
        "keyword".into(),
        taide_model::theme::SyntaxStyle {
            fg: "#ABC".into(),
            bold: true,
            italic: false,
        },
    );
    imported.token_colors = Some(vec![TokenColorRule {
        scope: vec!["keyword.control.synthetic".into()],
        settings: TokenColorSettings {
            foreground: Some("#ABC".into()),
            background: None,
            font_style: Some("bold italic".into()),
        },
    }]);
    imported.author = Some("Synthetic Author".into());
    imported.license = Some("Synthetic License".into());
    imported.source = Some("synthetic-source".into());
    theme_actions::theme_save(&state, imported.clone()).unwrap();
    let mut draft = Draft::load(&state, &imported.id, Mode::Edit, "ignored".into()).unwrap();
    assert_eq!(draft.current().name, imported.name);
    assert_eq!(draft.base().id, "taide-dark");
    assert!(!draft.has_unsaved_changes());
    assert_eq!(draft.changed_count(), 2);
    assert!(draft.color_changed(ColorDomain::Colors, "app.accent"));
    assert!(draft.syntax_changed("keyword"));
    let saved = draft.build().unwrap();
    assert_eq!(saved, imported);
    let preview = draft.preview();
    assert_eq!(preview.token_colors, imported.token_colors);
    assert_eq!(preview.author, imported.author);
    assert_eq!(preview.syntax_overrides, ["keyword"]);
    assert!(preview.warnings.is_empty());
    theme_actions::theme_save(&state, saved).unwrap();
    let round_trip = theme_actions::theme_get(&state, imported.id.clone()).unwrap();
    assert_eq!(round_trip.token_colors, imported.token_colors);
    assert_eq!(round_trip.author, imported.author);
    assert_eq!(round_trip.license, imported.license);
    assert_eq!(round_trip.source, imported.source);
    assert_eq!(round_trip.colors, draft.current().colors);
    assert_eq!(round_trip.syntax, draft.current().syntax);
    draft.rename("Renamed".into());
    assert!(draft.has_unsaved_changes());
    assert_eq!(draft.changed_count(), 2);
    draft.rename(imported.name.clone());
    assert!(!draft.has_unsaved_changes());
    draft
        .set_color(ColorDomain::Terminal, "red", " TRANSPARENT ".into())
        .unwrap();
    draft
        .set_syntax(
            "keyword",
            SyntaxPatch {
                italic: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(draft.is_valid());
    assert!(draft.has_unsaved_changes());
    assert_eq!(draft.changed_count(), 3);
    assert!(draft.current().syntax["keyword"].bold);
    assert!(draft.current().syntax["keyword"].italic);
    draft.reset_color(ColorDomain::Terminal, "red").unwrap();
    draft
        .set_syntax(
            "keyword",
            SyntaxPatch {
                italic: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(!draft.has_unsaved_changes());
    draft
        .reset_color(ColorDomain::Colors, "app.accent")
        .unwrap();
    draft.reset_syntax("keyword").unwrap();
    assert_eq!(draft.changed_count(), 0);
    assert!(draft.has_unsaved_changes());
    assert!(draft.build().unwrap().colors.is_empty());
    assert!(draft.build().unwrap().syntax.is_empty());
    assert!(draft.preview().syntax_overrides.is_empty());
    assert_eq!(draft.preview().token_colors, imported.token_colors);
    let before = draft.clone();
    assert!(
        draft
            .set_color(ColorDomain::Colors, "missing", "#fff".into())
            .is_err()
    );
    assert!(draft.set_syntax("missing", SyntaxPatch::default()).is_err());
    assert!(draft.reset_color(ColorDomain::Terminal, "missing").is_err());
    assert!(draft.reset_syntax("missing").is_err());
    assert_eq!(draft, before);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_theme_draft는_builtin_상속과_식별자_색상_저장경계를_검사한다() {
    let directory = std::env::temp_dir().join(format!(
        "taide-native-theme-draft-boundary-{}",
        ProjectId::new()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let state = AppState::new(AppPaths::new(directory.clone()));
    let themes = theme_actions::theme_list(&state).unwrap();
    for summary in &themes {
        assert!(summary.builtin);
        let mut draft = Draft::load(
            &state,
            &summary.id,
            Mode::Create,
            format!("{} copy", summary.name),
        )
        .unwrap();
        assert_eq!(draft.base().id, summary.id);
        assert!(!draft.has_unsaved_changes());
        assert_eq!(draft.changed_count(), 0);
        assert!(draft.is_valid());
        assert_eq!(draft.preview().token_colors, draft.base().token_colors);
        let saved = draft.build().unwrap();
        assert!(saved.token_colors.is_none());
        assert!(saved.colors.is_empty());
        assert!(saved.syntax.is_empty());
        assert!(saved.terminal.is_empty());
        theme_actions::theme_save(&state, saved.clone()).unwrap();
        let resolved = theme_actions::theme_get(&state, saved.id.clone()).unwrap();
        assert_eq!(resolved.colors, draft.current().colors);
        assert_eq!(resolved.syntax, draft.current().syntax);
        assert_eq!(resolved.terminal, draft.current().terminal);
        assert_eq!(resolved.token_colors, draft.current().token_colors);
        assert_eq!(
            Draft::load(&state, &summary.id, Mode::Edit, String::new())
                .unwrap_err()
                .kind(),
            AppErrorKind::InvalidArgument
        );
        draft.rename(" \t ".into());
        assert!(!draft.is_valid());
        assert!(draft.build().is_err());
    }
    let mut draft =
        Draft::load(&state, "taide-dark", Mode::Create, "Color boundary".into()).unwrap();
    for value in ["#abc", "#AbC123", "#12345678", " transparent ", "#abc\n"] {
        draft
            .set_color(ColorDomain::Colors, "app.background", value.into())
            .unwrap();
        assert!(draft.is_valid(), "{value}");
    }
    for value in [
        "#abcd",
        "#12345",
        "#1234567",
        "#123456789",
        "#ＡＢＣ",
        "red",
        "$palette",
        "#ggg",
        "",
    ] {
        draft
            .set_color(ColorDomain::Colors, "app.background", value.into())
            .unwrap();
        assert!(!draft.is_valid(), "{value}");
        assert_eq!(
            draft.build().unwrap_err().kind(),
            AppErrorKind::InvalidArgument
        );
    }
    draft
        .reset_color(ColorDomain::Colors, "app.background")
        .unwrap();
    draft
        .set_syntax(
            "keyword",
            SyntaxPatch {
                fg: Some("bad".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(!draft.is_valid());
    draft.reset_syntax("keyword").unwrap();
    draft
        .set_color(ColorDomain::Terminal, "red", "bad".into())
        .unwrap();
    assert!(!draft.is_valid());
    for id in ["../outside", "a/b", "a\\b", "", "..", "."] {
        assert_eq!(
            Draft::load(&state, id, Mode::Create, "name".into())
                .unwrap_err()
                .kind(),
            AppErrorKind::InvalidArgument
        );
    }
    assert_eq!(unique_id("  My __ THEME!  ", &[]), "my-theme");
    assert_eq!(unique_id("한글", &[]), "custom-theme");
    let existing = [ThemeSummary {
        id: "my-theme".into(),
        name: "occupied".into(),
        theme_type: ThemeType::Dark,
        builtin: false,
    }];
    let generated = unique_id("My Theme", &existing);
    assert!(generated.starts_with("my-theme-"));
    assert_eq!(generated.len(), "my-theme-".len() + ID_SUFFIX_LENGTH);
    assert!(
        generated["my-theme-".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
    taide_infra::root_guard::ensure_safe_component(&generated).unwrap();
    assert_eq!(builtin_id(ThemeType::Light), "taide-light");
    state.begin_shutdown();
    assert_eq!(
        Draft::load(&state, "taide-dark", Mode::Create, "name".into())
            .unwrap_err()
            .kind(),
        AppErrorKind::Forbidden
    );
    std::fs::remove_dir_all(directory).unwrap();
}
