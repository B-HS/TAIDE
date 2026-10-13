use taide_model::file::FileSizeTier;
use taide_native_editor::emmet::{
    CompletionLimits, ConversionLimits, MarkupLimits, Syntax, complete_document,
};
use taide_native_editor::language_configuration::LineSyntax;
use taide_native_editor::lsp::position_to_byte;
use taide_native_editor::store::EditorStore;
use taide_native_editor::syntax::TokenKind;
use taide_native_editor::view::ViewId;
use uuid::Uuid;

use super::{Request, State, cache};

const LIMITS: CompletionLimits = CompletionLimits {
    parse: MarkupLimits {
        max_bytes: 16 * 1024,
        max_nodes: 2048,
        max_nesting: 32,
    },
    conversion: ConversionLimits {
        max_repeats: 1000,
        max_nodes: 8192,
        max_bytes: 256 * 1024,
        max_nesting: 32,
    },
    max_candidates: 512,
    max_bytes: 2 * 1024 * 1024,
};
const HTML_TRIGGERS: &[&str] = &[
    "!", ".", "}", ":", "*", "$", "]", "/", ">", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
];
const CSS_TRIGGERS: &[&str] = &[
    ":", "!", "-", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
];
const RANDOM_BYTES: usize = size_of::<u32>();

pub(super) fn triggers(language: &str, enabled: bool) -> &'static [&'static str] {
    if !enabled {
        return &[];
    }
    match language {
        "html" => HTML_TRIGGERS,
        "css" | "scss" => CSS_TRIGGERS,
        _ => &[],
    }
}

fn allowed(request: &Request, syntax: &dyn LineSyntax, enabled: bool) -> bool {
    if !enabled
        || !matches!(
            request.snapshot.metadata.tier,
            FileSizeTier::Normal | FileSizeTier::Large
        )
        || Syntax::for_language(&request.snapshot.metadata.language_id).is_none()
        || request
            .selection
            .selections
            .iter()
            .any(|selection| selection.anchor != selection.head)
    {
        return false;
    }
    let Ok(byte) = position_to_byte(&request.snapshot, request.position) else {
        return false;
    };
    let start = request
        .snapshot
        .rope
        .line_to_byte(request.position.line as usize);
    syntax
        .accurate_tokens(&request.snapshot, request.position.line as usize)
        .is_some_and(|tokens| {
            tokens
                .iter()
                .rev()
                .find(|token| token.start_byte < byte - start)
                .is_some_and(|token| token.kind == TokenKind::Other)
        })
}

impl State {
    pub(super) fn supply_emmet(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        syntax: &dyn LineSyntax,
        enabled: bool,
    ) -> bool {
        let Some(request) = self.request(store, view).cloned() else {
            return false;
        };
        let enabled = allowed(&request, syntax, enabled);
        let entry = self.entries.get_mut(&view).unwrap();
        if entry.choice.is_some() || entry.emmet_context == Some((request.token, enabled)) {
            return false;
        }
        let items = if enabled {
            complete_document(&request.snapshot, request.position, LIMITS, &mut || {
                let bytes: [u8; RANDOM_BYTES] = Uuid::new_v4().as_bytes()[..RANDOM_BYTES]
                    .try_into()
                    .unwrap();
                u32::from_le_bytes(bytes)
            })
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        entry.emmet_context = Some((request.token, enabled));
        if items == entry.emmet {
            return false;
        }
        let byte = request.selection.selections[request.selection.primary].head;
        let rebased = entry
            .groups
            .values_mut()
            .try_for_each(|group| {
                cache::rebase_candidates(
                    store,
                    &request.snapshot,
                    byte,
                    &mut group.candidates.items,
                )
            })
            .and_then(|()| {
                cache::rebase_candidates(store, &request.snapshot, byte, &mut entry.snippets)
            })
            .and_then(|()| {
                cache::rebase_candidates(store, &request.snapshot, byte, &mut entry.words)
            });
        if rebased.is_err() {
            self.close(view);
            return true;
        }
        entry.emmet = items;
        entry.model = None;
        entry.preview = None;
        entry.documents.clear();
        entry.colors.clear();
        entry.widget_token = Uuid::new_v4();
        true
    }
}
