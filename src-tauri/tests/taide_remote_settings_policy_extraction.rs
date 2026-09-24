use taide_model::settings::{Settings, SettingsPatch};

#[test]
fn 원격_설정_패치는_지속적_권한_확장_필드를_제거한다() {
    let patch = SettingsPatch {
        remote_password_only_login: Some(true),
        remote_allowed_hosts: Some(vec!["attacker.example.com".to_string()]),
        shell_override: Some("/tmp/evil.sh".to_string()),
        ai_omlx_base_url: Some("http://attacker.example.com".to_string()),
        remote_access_enabled: Some(false),
        editor_font_size: Some(18),
        ..Default::default()
    };

    let filtered = taide_remote::policy::strip_remote_gated_settings_patch(patch);

    assert_eq!(filtered.remote_password_only_login, None);
    assert_eq!(filtered.remote_allowed_hosts, None);
    assert_eq!(filtered.shell_override, None);
    assert_eq!(filtered.ai_omlx_base_url, None);
    assert_eq!(filtered.remote_access_enabled, Some(false));
    assert_eq!(filtered.editor_font_size, Some(18));
}

#[test]
fn 원격_설정_전체_쓰기에서도_보호_필드는_현재값으로_복원된다() {
    let current = Settings {
        remote_password_only_login: true,
        remote_allowed_hosts: vec!["trusted.example.com".to_string()],
        shell_override: Some("/bin/zsh".to_string()),
        ai_omlx_base_url: Some("http://localhost:8000".to_string()),
        ..Settings::default()
    };
    let next = Settings {
        remote_password_only_login: false,
        remote_allowed_hosts: vec!["attacker.example.com".to_string()],
        shell_override: Some("/tmp/evil.sh".to_string()),
        ai_omlx_base_url: Some("http://attacker.example.com".to_string()),
        editor_font_size: 18,
        ..Settings::default()
    };

    let filtered = taide_remote::policy::strip_remote_gated_settings(next, &current);

    assert_eq!(filtered.remote_password_only_login, current.remote_password_only_login);
    assert_eq!(filtered.remote_allowed_hosts, current.remote_allowed_hosts);
    assert_eq!(filtered.shell_override, current.shell_override);
    assert_eq!(filtered.ai_omlx_base_url, current.ai_omlx_base_url);
    assert_eq!(filtered.editor_font_size, 18);
}
