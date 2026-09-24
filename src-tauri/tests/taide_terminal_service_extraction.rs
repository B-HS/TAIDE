use taide_lib::domain::terminal::service as legacy_service;
use taide_model::terminal::ShellProfile;

#[test]
fn terminal_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_profiles: fn() -> Vec<ShellProfile> = taide_terminal::service::list_shell_profiles;
    let legacy_profiles: fn() -> Vec<ShellProfile> = legacy_service::list_shell_profiles;
    assert!(std::ptr::fn_addr_eq(extracted_profiles, legacy_profiles));

    assert_eq!(
        taide_terminal::service::parse_terminal_path("src/main.rs:4:2"),
        legacy_service::parse_terminal_path("src/main.rs:4:2")
    );

    let extracted_ring: fn(usize) -> taide_terminal::service::ScrollbackRing = taide_terminal::service::ScrollbackRing::new;
    let legacy_ring: fn(usize) -> legacy_service::ScrollbackRing = legacy_service::ScrollbackRing::new;
    assert!(std::ptr::fn_addr_eq(extracted_ring, legacy_ring));
}
