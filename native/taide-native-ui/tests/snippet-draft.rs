use serde_json::Value;
use taide_model::snippet::{SnippetFile, SnippetMap};
use taide_native_ui::{
    snippet_draft::{self, Draft, Validation},
    snippet_editor_state::{DiscardTarget, Navigation, State},
};

fn text(value: &Value, field: &str) -> String {
    value[field]
        .as_str()
        .expect("fixture field must be a string")
        .into()
}

fn drafts(value: &Value) -> Vec<Draft> {
    value
        .as_array()
        .expect("fixture drafts must be an array")
        .iter()
        .map(|value| Draft {
            id: text(value, "id"),
            name: text(value, "name"),
            prefix: text(value, "prefix"),
            body: text(value, "body"),
            description: text(value, "description"),
            scope: text(value, "scope"),
        })
        .collect()
}

fn file(name: &str, content: &str) -> SnippetFile {
    SnippetFile {
        file_name: name.into(),
        snippets: serde_json::from_str(content).unwrap(),
    }
}

#[test]
fn 원본_typescript의_json_검증_정렬_공백_초안_변경과_동일하다() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/snippet-draft-parity.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let drafts = drafts(&case["drafts"]);
        assert_eq!(
            snippet_draft::content(&drafts).unwrap(),
            text(case, "content")
        );
        let valid = drafts.iter().map(Draft::is_valid).collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(valid).unwrap(), case["valid"]);
        let incomplete = case["incomplete"].as_u64().unwrap();
        let expected = if incomplete > 0 {
            Err(Validation::Incomplete(usize::try_from(incomplete).unwrap()))
        } else if case["duplicate"].as_bool().unwrap() {
            Err(Validation::DuplicateNames)
        } else {
            Ok(())
        };
        assert_eq!(snippet_draft::validate(&drafts), expected);
    }
    for name in fixture["names"].as_array().unwrap() {
        let normalized = snippet_draft::global_file_name(&text(name, "name"));
        assert_eq!(normalized, text(name, "normalized"));
        assert_eq!(
            snippet_draft::is_safe_file_name(&normalized),
            name["safe"].as_bool().unwrap()
        );
    }
    let saved: SnippetMap = serde_json::from_value(fixture["saved"].clone()).unwrap();
    let loaded = snippet_draft::from_snippets(&saved, String::new);
    let fields = loaded
        .iter()
        .map(|draft| {
            serde_json::json!({
                "name": draft.name, "prefix": draft.prefix, "body": draft.body,
                "description": draft.description, "scope": draft.scope,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(serde_json::to_value(fields).unwrap(), fixture["loaded"]);
    for case in fixture["dirty"].as_array().unwrap() {
        assert_eq!(
            snippet_draft::has_unsaved_changes(&drafts(&case["drafts"]), &saved),
            case["dirty"].as_bool().unwrap()
        );
    }
}

#[test]
fn 편집기_재조회가_초안을_덮지_않고_폐기와_행_삭제를_한번만_소비한다() {
    let first = file("rust.json", r#"{"A":{"prefix":"a","body":"one"}}"#);
    let global = file("common.code-snippets", "{}");
    let mut state = State::default();
    assert_eq!(
        state.request_select(first.file_name.clone()),
        Navigation::Selected
    );
    assert!(state.drafts().is_none());
    assert!(!state.append_entry());
    state.set_files(vec![first.clone(), global.clone()]);
    assert_eq!(state.drafts().unwrap().len(), 1);
    assert!(!state.show_scope());
    let id = state.drafts().unwrap()[0].id.clone();
    state.drafts_mut().unwrap()[0].body = "edited".into();
    assert!(state.has_unsaved_changes());
    assert_eq!(
        state.request_select(first.file_name.clone()),
        Navigation::Unchanged
    );
    assert_eq!(
        state.request_select(global.file_name.clone()),
        Navigation::ConfirmDiscard
    );
    assert_eq!(
        state.pending_discard(),
        Some(&DiscardTarget::Select(global.file_name.clone()))
    );
    state.cancel_discard();
    assert_eq!(state.confirm_discard(), Navigation::Unchanged);
    state.set_files(vec![first.clone(), global.clone()]);
    assert_eq!(state.drafts().unwrap()[0].id, id);
    assert_eq!(state.drafts().unwrap()[0].body, "edited");
    assert_eq!(state.request_close(), Navigation::ConfirmDiscard);
    assert_eq!(state.confirm_discard(), Navigation::Closed);
    assert_eq!(state.confirm_discard(), Navigation::Unchanged);
    assert_eq!(
        state.request_select(global.file_name.clone()),
        Navigation::ConfirmDiscard
    );
    assert_eq!(state.confirm_discard(), Navigation::Selected);
    assert!(state.show_scope());
    assert!(state.drafts().unwrap().is_empty());
    assert!(state.append_entry());
    let blank_id = state.drafts().unwrap()[0].id.clone();
    assert_ne!(blank_id, id);
    assert!(state.has_unsaved_changes());
    assert_eq!(state.validate_save(), Ok(()));
    assert_eq!(
        state.save_content().unwrap(),
        Some((global.file_name.clone(), "{}".into()))
    );
    state.drafts_mut().unwrap()[0].description = "unfinished".into();
    assert_eq!(state.validate_save(), Err(Validation::Incomplete(1)));
    state.delete_entry = Some(blank_id);
    assert_eq!(state.pending_delete_entry_name(), "");
    state.confirm_delete_entry();
    state.confirm_delete_entry();
    assert!(state.drafts().unwrap().is_empty());
    assert!(!state.has_unsaved_changes());
    assert_eq!(state.request_close(), Navigation::Closed);

    state.new_file.open = true;
    assert_eq!(state.new_file.option(), "rust");
    assert!(!state.new_file.can_create(state.files()));
    assert!(!state.new_file.select("../../bad"));
    assert_eq!(state.new_file.option(), "rust");
    assert!(state.new_file.select("global"));
    assert!(!state.new_file.can_create(state.files()));
    state.new_file.global_name = "\u{feff} custom \u{feff}".into();
    assert_eq!(state.new_file.file_name(), "custom.code-snippets");
    assert!(state.new_file.can_create(state.files()));
    state.new_file.open = false;
    state.new_file.open = true;
    assert_eq!(state.new_file.file_name(), "custom.code-snippets");
    state.new_file.global_name = "C:bad".into();
    assert!(!state.new_file.can_create(state.files()));
    state.new_file.global_name = "common".into();
    assert!(!state.new_file.can_create(state.files()));
    state.delete_file_open = true;
    state.deleted_file();
    assert!(!state.delete_file_open);
    assert!(state.selected_file_name().is_none());
    assert!(state.drafts().is_none());
    assert_eq!(state.request_close(), Navigation::Closed);
}
