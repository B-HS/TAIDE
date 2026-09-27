use std::path::PathBuf;

use taide_model::error::AppErrorKind;
use taide_model::paths::AppPaths;
use taide_runtime::{locale_actions, theme_actions, AppState};
use uuid::Uuid;

struct Fixture {
    dir: PathBuf,
    state: AppState,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-appearance-actions-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("fixture 생성");
        let state = AppState::new(AppPaths::new(dir.clone()));
        Self { dir, state }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).expect("fixture 정리");
    }
}

#[test]
fn 명시_테마는_live_설정과_사용자_테마를_소비하고_서비스_오류를_보존한다() {
    let fixture = Fixture::new();
    let mut custom = taide_theme::service::builtin_dark();
    custom.id = "fixture-theme".to_string();
    custom.name = "fixture".to_string();
    taide_theme::service::save_theme(&fixture.state.paths, &custom).unwrap();
    fixture.state.settings.write().follow_system_theme = false;
    fixture.state.settings.write().theme_id = custom.id.clone();
    assert_eq!(
        theme_actions::theme_get_current(&fixture.state, "light").unwrap(),
        taide_theme::service::load_theme(&fixture.state.paths, &custom.id).unwrap()
    );
    fixture.state.settings.write().theme_id = taide_theme::service::BUILTIN_LIGHT_ID.to_string();
    assert_eq!(
        theme_actions::theme_get_current(&fixture.state, "dark").unwrap().id,
        taide_theme::service::BUILTIN_LIGHT_ID
    );
    for id in ["fixture-missing", "../fixture-outside"] {
        fixture.state.settings.write().theme_id = id.to_string();
        assert_eq!(
            theme_actions::theme_get_current(&fixture.state, "light").unwrap_err().kind(),
            taide_theme::service::load_theme(&fixture.state.paths, id).unwrap_err().kind()
        );
    }
}

#[tokio::test]
async fn 시스템_테마_선택은_기존_fallback을_유지하고_mutation_잠금을_취득하지_않는다() {
    let fixture = Fixture::new();
    fixture.state.settings.write().follow_system_theme = true;
    fixture.state.settings.write().theme_id = "fixture-missing".to_string();
    let guard = fixture.state.begin_mutation().await;
    for system_theme in ["light", "Light", "dark", "", "unknown"] {
        assert_eq!(
            theme_actions::theme_get_current(&fixture.state, system_theme).unwrap().id,
            taide_theme::service::builtin_id_for_system(system_theme)
        );
    }
    assert!(fixture.state.settings.read().follow_system_theme);
    assert_eq!(fixture.state.settings.read().theme_id, "fixture-missing");
    drop(guard);
}

#[test]
fn 명시_언어는_live_설정과_사용자_pack_및_없는_언어_fallback을_보존한다() {
    let fixture = Fixture::new();
    let mut custom = taide_locale::service::builtin_ko();
    custom.id = "fixture-language".to_string();
    custom.name = "fixture".to_string();
    taide_locale::service::save_locale(&fixture.state.paths, &custom).unwrap();
    fixture.state.settings.write().language = custom.id.clone();
    assert_eq!(
        locale_actions::locale_get_current(&fixture.state, "en-US").unwrap(),
        taide_locale::service::load_locale(&fixture.state.paths, &custom.id).unwrap()
    );
    fixture.state.settings.write().language = taide_locale::service::BUILTIN_JA_ID.to_string();
    assert_eq!(
        locale_actions::locale_get_current(&fixture.state, "ko-KR").unwrap().id,
        taide_locale::service::BUILTIN_JA_ID
    );
    fixture.state.settings.write().language = "fixture-missing".to_string();
    assert_eq!(
        locale_actions::locale_get_current(&fixture.state, "ko-KR").unwrap().id,
        taide_locale::service::BUILTIN_EN_ID
    );
    fixture.state.settings.write().language = custom.id.clone();
    std::fs::write(fixture.state.paths.locales_dir().join(format!("{}.json", custom.id)), "{").unwrap();
    assert_eq!(
        locale_actions::locale_get_current(&fixture.state, "en-US").unwrap_err().kind(),
        taide_locale::service::load_locale(&fixture.state.paths, &custom.id)
            .unwrap_err()
            .kind()
    );
    assert_ne!(
        locale_actions::locale_get_current(&fixture.state, "en-US").unwrap_err().kind(),
        AppErrorKind::NotFound
    );
}

#[tokio::test]
async fn 시스템_언어는_대소문자와_짧은_문자열_및_fallback을_유지하고_설정을_바꾸지_않는다() {
    let fixture = Fixture::new();
    fixture.state.settings.write().language = "system".to_string();
    let guard = fixture.state.begin_mutation().await;
    for system_language in ["ko-KR", "JA-jp", "fr-FR", "", "한"] {
        let expected = taide_locale::service::resolve_language(&fixture.state.paths, "system", system_language);
        assert_eq!(
            locale_actions::locale_get_current(&fixture.state, system_language).unwrap().id,
            expected
        );
    }
    assert_eq!(fixture.state.settings.read().language, "system");
    drop(guard);
}

#[test]
fn 현재_테마와_언어_command는_runtime_selector에_위임한다() {
    let theme = include_str!("../src/domain/theme/commands.rs");
    let locale = include_str!("../src/domain/locale/commands.rs");
    assert!(theme.contains("theme_actions::theme_get_current(&state, &system_theme)"));
    assert!(locale.contains("locale_actions::locale_get_current(&state, &system_language)"));
    assert!(!theme.contains("state.settings.read()"));
    assert!(!locale.contains("state.settings.read()"));
}
