use taide_lib::domain::ai::prompt::{AUTO_TAB_PROMPT_ID, COMMIT_MESSAGE_PROMPT_ID, INLINE_EDIT_PROMPT_ID};
use taide_lib::domain::app::types::{AppFileTarget, AppInfo, PerfSnapshot, PromptTemplateId};
use taide_model::app::{AppFileTarget as ModelAppFileTarget, PromptTemplateId as ModelPromptTemplateId};

fn model_target(target: AppFileTarget) -> ModelAppFileTarget {
    target
}

#[test]
fn 프롬프트_id는_기존_파일명과_공개_타입을_유지한다() {
    let cases = [
        (PromptTemplateId::AutoTabDefault, AUTO_TAB_PROMPT_ID, "auto-tab-default"),
        (PromptTemplateId::InlineEditDefault, INLINE_EDIT_PROMPT_ID, "inline-edit-default"),
        (
            PromptTemplateId::CommitMessageDefault,
            COMMIT_MESSAGE_PROMPT_ID,
            "commit-message-default",
        ),
    ];

    for (facade, old_constant, expected) in cases {
        let model: ModelPromptTemplateId = facade;
        assert_eq!(model.as_str(), expected);
        assert_eq!(old_constant, expected);
        assert_eq!(serde_json::to_value(model).expect("프롬프트 ID 직렬화"), expected);
    }
}

#[test]
fn 앱_소유_파일_대상의_기존_wire와_공개_타입을_유지한다() {
    let settings = model_target(AppFileTarget::Settings);
    assert_eq!(
        serde_json::to_value(settings).expect("설정 대상 직렬화"),
        serde_json::json!({ "kind": "settings" })
    );

    let wire = serde_json::json!({ "kind": "prompt", "id": "inline-edit-default" });
    let target: ModelAppFileTarget = serde_json::from_value(wire.clone()).expect("기존 프롬프트 대상");
    let _: AppFileTarget = target;
    assert_eq!(serde_json::to_value(target).expect("프롬프트 대상 직렬화"), wire);

    let _: taide_model::app::AppInfo = AppInfo {
        name: "TAIDE".into(),
        version: "0.2.6".into(),
        platform: "macos".into(),
        arch: "aarch64".into(),
    };
    let _: taide_model::app::PerfSnapshot = PerfSnapshot {
        enabled: false,
        entries: vec![],
        counters: vec![],
    };
}
