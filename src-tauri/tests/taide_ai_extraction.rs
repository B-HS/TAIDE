use std::any::TypeId;

use taide_infra::secret::SecretStore;
use taide_lib::domain::ai::{prompt as legacy_prompt, providers as legacy_providers, service as legacy_service};
use taide_model::ai::{AiPromptTemplate, AiProviderId};
use taide_model::error::AppResult;

#[test]
fn ai_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_prompt: fn() -> AiPromptTemplate = taide_ai::prompt::bundled_prompt_template;
    let legacy_prompt: fn() -> AiPromptTemplate = legacy_prompt::bundled_prompt_template;
    assert!(std::ptr::fn_addr_eq(extracted_prompt, legacy_prompt));
    assert_eq!(extracted_prompt(), legacy_prompt());

    let extracted_clear: fn(&dyn SecretStore, AiProviderId) -> AppResult<()> = taide_ai::service::clear_token;
    let legacy_clear: fn(&dyn SecretStore, AiProviderId) -> AppResult<()> = legacy_service::clear_token;
    assert!(std::ptr::fn_addr_eq(extracted_clear, legacy_clear));

    assert_eq!(
        TypeId::of::<taide_ai::providers::codex::CodexProvider>(),
        TypeId::of::<legacy_providers::codex::CodexProvider>()
    );
}
