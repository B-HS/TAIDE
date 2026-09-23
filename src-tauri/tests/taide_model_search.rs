use taide_lib::domain::search::types::{ReplaceSkipReason, SearchFileMatches, SearchQuery, SearchReplaceResult};
use taide_model::search::{SearchQuery as ModelSearchQuery, SearchReplaceResult as ModelSearchReplaceResult};

fn model_query(query: SearchQuery) -> ModelSearchQuery {
    query
}

#[test]
fn 구버전_검색_쿼리의_기본값과_공개_타입이_유지된다() {
    let wire = serde_json::json!({ "text": "needle" });
    let query: SearchQuery = serde_json::from_value(wire).expect("기존 검색 쿼리");
    let model = model_query(query);
    assert!(!model.case_sensitive);
    assert!(!model.whole_word);
    assert!(!model.regex);
    assert_eq!(model.context_lines, 0);
    assert!(model.respect_gitignore);
    assert!(model.scope_dir.is_none());
    assert_eq!(serde_json::to_value(&model).expect("검색 쿼리 직렬화")["respectGitignore"], true);
}

#[test]
fn 검색_결과와_치환_실패_사유는_기존_wire를_유지한다() {
    let result: ModelSearchReplaceResult = serde_json::from_value(serde_json::json!({
        "changedFiles": 0,
        "replacedMatches": 0,
        "skipped": [{ "path": "binary.png", "reason": "notUtf8" }],
        "skippedCount": 1
    }))
    .expect("기존 검색 치환 결과");
    let facade: SearchReplaceResult = result;
    assert_eq!(facade.skipped[0].reason, ReplaceSkipReason::NotUtf8);
    assert_eq!(serde_json::to_value(facade).expect("검색 결과 직렬화")["skippedCount"], 1);

    let matches: taide_model::search::SearchFileMatches = serde_json::from_value(serde_json::json!({
        "path": "main.rs",
        "matches": [{
            "line": 1,
            "column": 2,
            "preview": "needle",
            "matchStart": 1,
            "matchEnd": 3,
            "before": [],
            "after": []
        }]
    }))
    .expect("기존 검색 결과 채널");
    let _: SearchFileMatches = matches;
}
