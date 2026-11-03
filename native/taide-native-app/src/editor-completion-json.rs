use std::sync::{Arc, OnceLock};

use taide_lsp::native::protocol::lsp_types::{
    CompletionItem, CompletionList, CompletionResponse, CompletionTextEdit, InsertTextFormat,
    TextEdit,
};
use taide_model::file::FileSizeTier;
use taide_native_editor::completion::Candidates;
use taide_native_editor::document::{DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::json_completion::{self, Environment};
use taide_native_editor::lsp::{LspRange, byte_to_position, position_to_byte};
use taide_native_syntax::{JsonSchemaPatternCompiler, JsonSchemaValidationPatternCompiler};
use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;
use uuid::Uuid;

use super::Request;
use crate::editor_json_validation::{SCHEMA_LIMITS, contributions, resource_uri};
use crate::editor_symbols::ProviderIdentity;

const WORKER_NAME: &str = "native-json-completion";
const MAX_ITEMS: usize = 16 * 1024;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_WORK: usize = 64 * 1024 * 1024;
const MAX_DEPTH: usize = 8192;
pub(super) const ORDINAL: usize = usize::MAX;
const LIMITS: json_completion::Limits = json_completion::Limits {
    document: SCHEMA_LIMITS,
    max_items: MAX_ITEMS,
    max_text_bytes: MAX_TEXT_BYTES,
    max_work: MAX_WORK,
    max_depth: MAX_DEPTH,
};

pub(super) struct Pending {
    pub token: Uuid,
    pub receiver: oneshot::Receiver<Result<Candidates, EditorError>>,
}

pub(crate) fn provider() -> ProviderIdentity {
    static PROVIDER: OnceLock<ProviderIdentity> = OnceLock::new();
    *PROVIDER.get_or_init(|| ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 0,
        capability_revision: 0,
    })
}

pub(crate) fn applies(snapshot: &DocumentSnapshot) -> bool {
    snapshot.metadata.language_id == "json"
        && snapshot.metadata.tier == FileSizeTier::Normal
        && !matches!(
            snapshot.key,
            DocumentKey::AppFile(_) | DocumentKey::Virtual(_)
        )
}

pub(super) fn spawn(
    request: &Request,
    tasks: &TaskSupervisor,
    repaint: Arc<dyn Fn() + Send + Sync>,
) -> Option<Pending> {
    if !applies(&request.snapshot) {
        return None;
    }
    let (sender, receiver) = oneshot::channel();
    let worker = request.clone();
    tasks.spawn_blocking_transient_handle(WORKER_NAME, move || {
        if worker.is_cancelled() {
            return;
        }
        let result = complete(&worker);
        if !worker.is_cancelled() && !sender.is_closed() {
            drop(sender.send(result));
            repaint();
        }
    })?;
    Some(Pending {
        token: request.token,
        receiver,
    })
}

fn complete(request: &Request) -> Result<Candidates, EditorError> {
    let snapshot = &request.snapshot;
    let text = snapshot.rope.to_string();
    let resource = resource_uri(&snapshot.key)?;
    let result = json_completion::complete(
        &text,
        position_to_byte(snapshot, request.position)?,
        Environment {
            resource: &resource,
            contributions: contributions()?,
            associations: &JsonSchemaPatternCompiler,
            patterns: &JsonSchemaValidationPatternCompiler,
            cancelled: &|| request.is_cancelled(),
        },
        LIMITS,
    )?;
    let items = result
        .items
        .into_iter()
        .map(|item| {
            let range = LspRange::new(
                byte_to_position(snapshot, item.range.start)?,
                byte_to_position(snapshot, item.range.end)?,
            );
            Ok(CompletionItem {
                label: item.label,
                kind: Some(item.kind),
                detail: item.detail,
                documentation: item.documentation,
                sort_text: item.sort_text,
                filter_text: item.filter_text,
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: item.insert_text,
                })),
                command: item.command,
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    Ok(Candidates::new(
        snapshot,
        request.position,
        request.word,
        Some(CompletionResponse::List(CompletionList {
            is_incomplete: result.is_incomplete,
            items,
        })),
    ))
}
