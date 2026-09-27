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

#[test]
fn theme_공개_action은_같은_저장_조회_삭제와_builtin_정책을_쓴다() {
    let fixture = Fixture::new();
    let mut custom = taide_theme::service::builtin_dark();
    custom.id = "fixture-public-theme".to_string();
    custom.name = "fixture public".to_string();
    let saved = theme_actions::theme_save(&fixture.state, custom.clone()).unwrap();
    assert_eq!(saved.id, custom.id);
    assert!(!saved.builtin);
    assert_eq!(
        theme_actions::theme_get(&fixture.state, custom.id.clone()).unwrap(),
        taide_theme::service::load_theme(&fixture.state.paths, &custom.id).unwrap()
    );
    assert!(theme_actions::theme_list(&fixture.state)
        .unwrap()
        .iter()
        .any(|entry| entry.id == custom.id));
    theme_actions::theme_delete(&fixture.state, custom.id.clone()).unwrap();
    assert_eq!(
        theme_actions::theme_get(&fixture.state, custom.id).unwrap_err().kind(),
        AppErrorKind::NotFound
    );
    assert_eq!(
        theme_actions::theme_delete(&fixture.state, taide_theme::service::BUILTIN_DARK_ID.to_string())
            .unwrap_err()
            .kind(),
        taide_theme::service::delete_theme(&fixture.state.paths, taide_theme::service::BUILTIN_DARK_ID)
            .unwrap_err()
            .kind()
    );
    assert_eq!(
        theme_actions::theme_get(&fixture.state, taide_theme::service::BUILTIN_LIGHT_ID.to_string()).unwrap(),
        taide_theme::service::load_theme(&fixture.state.paths, taide_theme::service::BUILTIN_LIGHT_ID).unwrap()
    );
}

#[test]
fn theme_검증_실패는_같은_오류이고_기존_파일을_보존한다() {
    let fixture = Fixture::new();
    let mut custom = taide_theme::service::builtin_dark();
    custom.id = "fixture-public-theme".to_string();
    theme_actions::theme_save(&fixture.state, custom.clone()).unwrap();
    let path = fixture.state.paths.themes_dir().join(format!("{}.json", custom.id));
    let original = std::fs::read(&path).unwrap();
    for id in ["../fixture-outside", "fixture-missing"] {
        assert_eq!(
            theme_actions::theme_get(&fixture.state, id.to_string()).unwrap_err().kind(),
            taide_theme::service::load_theme(&fixture.state.paths, id).unwrap_err().kind()
        );
        assert_eq!(
            theme_actions::theme_delete(&fixture.state, id.to_string()).unwrap_err().kind(),
            taide_theme::service::delete_theme(&fixture.state.paths, id).unwrap_err().kind()
        );
    }
    let mut invalid = custom;
    invalid.id = "../fixture-outside".to_string();
    assert_eq!(
        theme_actions::theme_save(&fixture.state, invalid.clone()).unwrap_err().kind(),
        taide_theme::service::save_theme(&fixture.state.paths, &invalid).unwrap_err().kind()
    );
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn locale_목록과_조회는_사용자_pack_및_경로_이탈_거부와_깨진_json_오류를_유지한다() {
    let fixture = Fixture::new();
    let mut custom = taide_locale::service::builtin_ko();
    custom.id = "fixture-public-language".to_string();
    taide_locale::service::save_locale(&fixture.state.paths, &custom).unwrap();
    assert_eq!(
        locale_actions::locale_list(&fixture.state).unwrap(),
        taide_locale::service::list_locales(&fixture.state.paths)
    );
    for id in [custom.id.as_str(), "en"] {
        assert_eq!(
            locale_actions::locale_get(&fixture.state, id.to_string()).unwrap(),
            taide_locale::service::load_locale(&fixture.state.paths, id).unwrap()
        );
    }
    for id in ["fixture-missing", "../fixture-outside"] {
        assert_eq!(
            locale_actions::locale_get(&fixture.state, id.to_string()).unwrap_err().kind(),
            taide_locale::service::load_locale(&fixture.state.paths, id).unwrap_err().kind()
        );
    }
    let outside_locale_dir = fixture.dir.join("fixture-outside.json");
    taide_infra::persist::write_json(&outside_locale_dir, &custom).unwrap();
    assert_eq!(
        locale_actions::locale_get(&fixture.state, "../fixture-outside".to_string())
            .unwrap_err()
            .kind(),
        AppErrorKind::InvalidArgument
    );
    assert!(outside_locale_dir.exists());
    std::fs::write(fixture.state.paths.locales_dir().join(format!("{}.json", custom.id)), "{").unwrap();
    assert_eq!(
        locale_actions::locale_get(&fixture.state, custom.id.clone()).unwrap_err().kind(),
        taide_locale::service::load_locale(&fixture.state.paths, &custom.id)
            .unwrap_err()
            .kind()
    );
}

#[test]
fn 나머지_테마_언어_command도_runtime에_위임한다() {
    for (source, module, names) in [
        (
            include_str!("../src/domain/theme/commands.rs"),
            "theme_actions",
            &["theme_list", "theme_get", "theme_save", "theme_delete"][..],
        ),
        (
            include_str!("../src/domain/locale/commands.rs"),
            "locale_actions",
            &["locale_list", "locale_get"][..],
        ),
    ] {
        for name in names {
            let body = source
                .split_once(&format!("pub async fn {name}("))
                .unwrap()
                .1
                .split_once("\n}")
                .unwrap()
                .0;
            assert!(body.contains(&format!("{module}::{name}(")), "{name}");
        }
    }
}
