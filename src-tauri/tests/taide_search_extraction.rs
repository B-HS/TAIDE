use taide_lib::constants::REFUSED_FILE_BYTES as LEGACY_REFUSED_FILE_BYTES;
use taide_lib::domain::search::service as legacy_service;
use taide_model::search::SearchQuery;

#[test]
fn 검색_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_glob: fn(&str, &str) -> bool = taide_search::service::glob_match;
    let legacy_glob: fn(&str, &str) -> bool = legacy_service::glob_match;
    assert!(std::ptr::fn_addr_eq(extracted_glob, legacy_glob));
    assert!(extracted_glob("*.rs", "main.rs"));

    let extracted_matches: fn(&str, &SearchQuery) -> Vec<(u32, u32)> = taide_search::service::find_matches_in_line;
    let legacy_matches: fn(&str, &SearchQuery) -> Vec<(u32, u32)> = legacy_service::find_matches_in_line;
    assert!(std::ptr::fn_addr_eq(extracted_matches, legacy_matches));

    let query: SearchQuery = serde_json::from_str(r#"{"text":"needle"}"#).expect("기본 검색 쿼리");
    assert_eq!(extracted_matches("needle", &query), legacy_matches("needle", &query));
    assert_eq!(taide_model::file::REFUSED_FILE_BYTES, LEGACY_REFUSED_FILE_BYTES);
}
