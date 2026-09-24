use std::path::Path;

use taide_infra::{clock, crypto, home, language, redact, shell_quote};
use taide_lib::infra;

#[test]
fn 독립_infra_타입과_기존_facade가_같다() {
    let _: Option<infra::language::LanguageOverlay> = None::<language::LanguageOverlay>;
    assert_eq!(infra::clock::MS_PER_SECOND, clock::MS_PER_SECOND);
}

#[test]
fn 독립_infra_도구와_기존_경로가_같은_결과를_낸다() {
    assert!(crypto::constant_time_eq(b"same", b"same"));
    assert_eq!(
        infra::crypto::constant_time_eq(b"same", b"other"),
        crypto::constant_time_eq(b"same", b"other")
    );
    assert_eq!(
        infra::home::expand_home("~/repo", Some("/home/user")),
        home::expand_home("~/repo", Some("/home/user"))
    );
    assert_eq!(infra::shell_quote::posix_quote("a'b"), shell_quote::posix_quote("a'b"));
    assert_eq!(
        infra::language::language_id_for_path(Path::new("main.rs"), &[]),
        language::language_id_for_path(Path::new("main.rs"), &[])
    );
    assert_eq!(
        infra::redact::mask_provider_error("request failed"),
        redact::mask_provider_error("request failed")
    );
}
