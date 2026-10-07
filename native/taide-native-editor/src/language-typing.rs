use std::ops::Range;

use crate::auto_closing::{
    AutoClosedPairs, closing_text, overtypes, pair_deletions, surrounding_close,
};
use crate::auto_indent::{electric_reindent, line_break, typed_reindent};
use crate::document::EditorError;
use crate::editing::{
    self, Mark, Plan, SEPARATE_STEP, Step, apply_step, ordered, previous_operation, typing_step,
    view_document,
};
use crate::indent::IndentOptions;
use crate::language_configuration::Language;
use crate::store::EditorStore;
use crate::view::{EditOperation, ViewId};

const LINE_FEED: char = '\n';

const AUTOMATIC_EDIT_STEP: Step = Step {
    operation: EditOperation::TypingOther,
    stop_before: true,
    stop_after: false,
};

pub struct Typing<'a> {
    pub language: Option<Language<'a>>,
    pub indent: IndentOptions,
    pub auto_closed: &'a mut AutoClosedPairs,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Keyboard,
    CompositionEnd,
}

fn type_character(
    store: &mut EditorStore,
    view: ViewId,
    character: char,
    language: Language<'_>,
    typing: &mut Typing<'_>,
    source: Source,
) -> Result<bool, EditorError> {
    if character == LINE_FEED && source == Source::Keyboard {
        return break_line(store, view, language, typing.indent);
    }
    typing.auto_closed.follow(store, view)?;
    let (current, document) = view_document(store, view)?;
    let selections = &current.selection.selections;
    let previous = previous_operation(&current, &document);
    let typed = character.to_string();
    let mut plan = Plan::new(&current.selection);
    let closing = closing_text(&document, selections, character, language);
    let reindented: Option<Vec<(Range<usize>, String)>> = (source == Source::Keyboard
        && closing.is_none())
    .then(|| {
        selections
            .iter()
            .map(|selection| {
                typed_reindent(
                    &document,
                    &ordered(selection),
                    character,
                    language,
                    typing.indent,
                )
            })
            .collect()
    })
    .flatten();
    let mut closes_pairs = false;
    let step = if let Some(edits) = reindented {
        for (index, (bytes, text)) in edits.into_iter().enumerate() {
            plan.replace(index, bytes, text);
        }
        AUTOMATIC_EDIT_STEP
    } else if overtypes(
        &document,
        selections,
        character,
        language,
        typing.auto_closed,
    ) {
        for (index, selection) in selections.iter().enumerate() {
            plan.replace(
                index,
                selection.head..selection.head + typed.len(),
                typed.clone(),
            );
        }
        match source {
            Source::Keyboard => typing_step(previous, EditOperation::TypingOther),
            Source::CompositionEnd => AUTOMATIC_EDIT_STEP,
        }
    } else if let Some(close) = &closing {
        for (index, selection) in selections.iter().enumerate() {
            if let Some(edit) = plan.edit(selection.head..selection.head, format!("{typed}{close}"))
            {
                let caret = Mark::BeforeEditEnd {
                    edit,
                    bytes: close.len(),
                };
                plan.marks[index] = (caret, caret);
            }
        }
        closes_pairs = true;
        AUTOMATIC_EDIT_STEP
    } else if let Some(close) = surrounding_close(&document, selections, character, language) {
        let mut surrounded = Plan::new(&current.selection);
        let is_surrounded = selections.iter().enumerate().all(|(index, selection)| {
            let range = ordered(selection);
            let Some(opened) = surrounded.edit(range.start..range.start, typed.clone()) else {
                return false;
            };
            surrounded.marks[index] = (
                Mark::AfterEdit(opened),
                Mark::Tracked {
                    offset: range.end,
                    sticks: true,
                },
            );
            surrounded
                .edit(range.end..range.end, close.into())
                .is_some()
        });
        if !is_surrounded {
            return editing::type_text(store, view, &typed);
        }
        plan = surrounded;
        match source {
            Source::Keyboard => SEPARATE_STEP,
            Source::CompositionEnd => AUTOMATIC_EDIT_STEP,
        }
    } else if let Some((bytes, text)) = (source == Source::Keyboard)
        .then_some(selections.as_slice())
        .and_then(|selections| match selections {
            [selection] if selection.anchor == selection.head => electric_reindent(
                &document,
                selection.head,
                character,
                language,
                typing.indent,
            ),
            _ => None,
        })
    {
        plan.replace(0, bytes, text);
        Step {
            operation: EditOperation::TypingOther,
            stop_before: false,
            stop_after: true,
        }
    } else {
        return editing::type_text(store, view, &typed);
    };
    if !plan.has_edits() {
        return editing::type_text(store, view, &typed);
    }
    let changed = apply_step(store, view, plan.transaction(&document, view), step)?;
    if closes_pairs && let Some(close) = &closing {
        let (edited, document) = view_document(store, view)?;
        let carets = edited.selection.selections.iter().map(|caret| caret.head);
        typing.auto_closed.record(
            &document,
            carets
                .clone()
                .map(|head| head..head + close.len())
                .collect(),
            carets
                .map(|head| head.saturating_sub(typed.len())..head + close.len())
                .collect(),
        );
    }
    typing.auto_closed.follow(store, view)?;
    Ok(changed)
}

fn break_line(
    store: &mut EditorStore,
    view: ViewId,
    language: Language<'_>,
    indent: IndentOptions,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let line_ending = document.metadata.line_ending.as_str();
    let mut plan = Plan::new(&current.selection);
    for (index, selection) in current.selection.selections.iter().enumerate() {
        let edit = line_break(&document, ordered(selection), language, indent, line_ending);
        let Some(applied) = plan.edit(edit.bytes, edit.text) else {
            continue;
        };
        let caret = if edit.caret_before_end == 0 {
            Mark::AfterEdit(applied)
        } else {
            Mark::BeforeEditEnd {
                edit: applied,
                bytes: edit.caret_before_end,
            }
        };
        plan.marks[index] = (caret, caret);
    }
    apply_step(
        store,
        view,
        plan.transaction(&document, view),
        AUTOMATIC_EDIT_STEP,
    )
}

pub fn type_text(
    store: &mut EditorStore,
    view: ViewId,
    text: &str,
    typing: &mut Typing<'_>,
) -> Result<bool, EditorError> {
    let Some(language) = typing.language else {
        return editing::type_text(store, view, text);
    };
    let mut changed = false;
    for character in text.chars() {
        changed |= type_character(store, view, character, language, typing, Source::Keyboard)?;
    }
    Ok(changed)
}

pub fn commit_composition(
    store: &mut EditorStore,
    view: ViewId,
    replaced: Range<usize>,
    text: &str,
    typing: &mut Typing<'_>,
) -> Result<bool, EditorError> {
    let mut characters = text.chars();
    let (Some(language), Some(character), None) =
        (typing.language, characters.next(), characters.next())
    else {
        return editing::compose_text(store, view, replaced, text);
    };
    let (current, _) = view_document(store, view)?;
    let replaces_only_selection = matches!(
        current.selection.selections.as_slice(),
        [selection] if ordered(selection) == replaced
    );
    if !replaces_only_selection {
        return editing::compose_text(store, view, replaced, text);
    }
    type_character(
        store,
        view,
        character,
        language,
        typing,
        Source::CompositionEnd,
    )
}

pub fn insert_line_break(
    store: &mut EditorStore,
    view: ViewId,
    typing: &mut Typing<'_>,
) -> Result<bool, EditorError> {
    let changed = match typing.language {
        Some(language) => break_line(store, view, language, typing.indent)?,
        None => editing::insert_line_break(store, view, typing.indent)?,
    };
    typing.auto_closed.follow(store, view)?;
    Ok(changed)
}

pub fn delete_backward(
    store: &mut EditorStore,
    view: ViewId,
    typing: &mut Typing<'_>,
) -> Result<bool, EditorError> {
    let Some(language) = typing.language else {
        return editing::delete_backward(store, view, typing.indent);
    };
    typing.auto_closed.follow(store, view)?;
    let (current, document) = view_document(store, view)?;
    let deletions = pair_deletions(
        &document,
        &current.selection.selections,
        language,
        typing.auto_closed,
    );
    let changed = match deletions {
        Some(ranges) => {
            let mut plan = Plan::new(&current.selection);
            for (index, bytes) in ranges.into_iter().enumerate() {
                plan.replace(index, bytes, String::new());
            }
            apply_step(
                store,
                view,
                plan.transaction(&document, view),
                Step {
                    operation: EditOperation::DeletingLeft,
                    stop_before: true,
                    stop_after: false,
                },
            )?
        }
        None => editing::delete_backward(store, view, typing.indent)?,
    };
    typing.auto_closed.follow(store, view)?;
    Ok(changed)
}
