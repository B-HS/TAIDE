use serde_json::Value;
use taide_model::snippet::SnippetFile;
use taide_native_ui::snippet_completion::collect;

#[test]
fn 원본_typescript_자동완성_후보의_언어_scope_숫자키_배열과_공백을_보존한다() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/snippet-completion-parity.json")).unwrap();
    let files: Vec<SnippetFile> = serde_json::from_value(fixture["files"].clone()).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let language = case["language"].as_str().unwrap();
        assert_eq!(
            serde_json::to_value(collect(&files, language)).unwrap(),
            case["expected"],
            "{language}"
        );
    }
}
