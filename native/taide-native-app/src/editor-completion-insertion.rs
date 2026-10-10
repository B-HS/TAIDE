use std::cell::RefCell;

use taide_native_editor::completion::PreparationOptions;
use taide_native_editor::document::EditorError;
use taide_native_editor::language_configuration::LanguageRules;
use taide_native_editor::snippet_insertion::PreparedSnippet;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{Selection, SelectionSet, ViewId};
use taide_native_syntax::MonacoSnippetTransforms;
use uuid::Uuid;

use super::State;
use super::variables::{Clock, Variables};

pub(crate) struct PreparationContext<'a> {
    pub model_path: &'a str,
    pub language: &'a dyn LanguageRules,
    pub clipboard: Option<&'a str>,
    pub clipboard_spread: bool,
    pub clock: &'a Clock,
}

pub(crate) struct PreparedCompletion {
    pub snippets: Vec<PreparedSnippet>,
    pub transforms: MonacoSnippetTransforms,
}

impl State {
    pub(crate) fn prepare_candidate(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: Uuid,
        index: usize,
        options: PreparationOptions,
        context: PreparationContext<'_>,
    ) -> Result<PreparedCompletion, EditorError> {
        self.request(store, view).ok_or(EditorError::Refused)?;
        if self
            .entries
            .get(&view)
            .is_none_or(|entry| entry.widget_token != token)
            || options.indent.tab_size == 0
        {
            return Err(EditorError::Refused);
        }
        let model = self.model(store, view)?.ok_or(EditorError::Refused)?;
        let candidate = model
            .borrow()
            .candidate(index)
            .cloned()
            .ok_or(EditorError::InvalidBoundary)?;
        let source = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(source.document)?;
        let ranges =
            candidate.replacement_ranges(&document, &source.selection, options.alternate)?;
        let selection = SelectionSet {
            primary: source.selection.primary,
            selections: ranges
                .into_iter()
                .zip(&source.selection.selections)
                .map(|(range, selection)| {
                    if selection.anchor > selection.head {
                        return Selection {
                            anchor: range.end,
                            head: range.start,
                        };
                    }
                    Selection {
                        anchor: range.start,
                        head: range.end,
                    }
                })
                .collect(),
        };
        let transforms = RefCell::new(MonacoSnippetTransforms::new(options.limits));
        let mut random = Uuid::new_v4;
        let mut variables = Variables {
            document: &document,
            selection: &selection,
            model_path: context.model_path,
            language: context.language,
            clipboard: context.clipboard,
            clipboard_spread: context.clipboard_spread,
            clock: context.clock,
            random: &mut random,
            max_bytes: options.limits.max_bytes,
        };
        let snippets = candidate.prepare(
            &document,
            &source.selection,
            options,
            |source, flags| transforms.borrow_mut().compile(source, flags),
            |cursor, variable| variables.resolve(cursor, variable, None),
            |_, transform, value| transforms.borrow_mut().evaluate(transform, value),
        )?;
        Ok(PreparedCompletion {
            snippets,
            transforms: transforms.into_inner(),
        })
    }
}
