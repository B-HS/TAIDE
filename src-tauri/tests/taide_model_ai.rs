use taide_lib::domain::ai::types::{AiInlineEditRequest, AiPromptTemplate, AiProviderId, AiTextResponse};
use taide_model::ai::{AiInlineEditRequest as ModelAiInlineEditRequest, AiPromptTemplate as ModelAiPromptTemplate};

#[test]
fn ai_provider와_구버전_편집_요청의_wire를_유지한다() {
    let provider: taide_model::ai::AiProviderId = serde_json::from_str("\"ollamaCloud\"").expect("기존 provider ID");
    let facade: AiProviderId = provider;
    assert_eq!(facade, AiProviderId::OllamaCloud);

    let legacy = serde_json::json!({
        "requestId": "req-1", "owner": "main", "selection": "before", "instruction": "edit",
        "language": "rust", "filePath": "/project/main.rs", "prefix": "", "suffix": ""
    });
    let model: ModelAiInlineEditRequest = serde_json::from_value(legacy).expect("기존 인라인 편집 요청");
    let facade: AiInlineEditRequest = model;
    assert_eq!(facade.owner, "main");
    assert!(facade.provider.is_none());
    assert!(facade.model.is_none());
    assert_eq!(serde_json::to_value(facade).expect("요청 직렬화")["filePath"], "/project/main.rs");
}

#[test]
fn 저장된_프롬프트_템플릿과_텍스트_응답_타입을_유지한다() {
    let legacy = serde_json::json!({
        "version": 1,
        "fim": { "prompt": "prefix", "suffix": "suffix" },
        "chat": { "system": "system", "user": "user" }
    });
    let model: ModelAiPromptTemplate = serde_json::from_value(legacy).expect("기존 프롬프트 템플릿");
    let facade: AiPromptTemplate = model;
    assert!(facade.fim.stop.is_empty());
    assert_eq!(facade.chat.user, "user");

    let response: taide_model::ai::AiTextResponse = serde_json::from_value(serde_json::json!({
        "requestId": "req-1", "text": null
    }))
    .expect("기존 취소 응답");
    let facade: AiTextResponse = response;
    assert!(facade.text.is_none());
}
