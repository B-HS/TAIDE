use crate::document::{DocumentSnapshot, EditorError};
use crate::editing::{Plan, SEPARATE_STEP, apply_step, ordered, view_document};
use crate::language_configuration::Language;
use crate::store::EditorStore;
use crate::view::{SelectionSet, ViewId};
use ropey::{Rope, RopeSlice};
use taide_model::file::{EditorConfigIndentStyle, EditorConfigOptions};

const DETECTION_LINE_LIMIT: usize = 10_000;
const TAB_SIZE_GUESSES: [usize; 7] = [2, 4, 6, 8, 3, 5, 7];
const MAX_TAB_SIZE_GUESS: usize = 8;
const TWO_SPACE_SIZE: usize = 2;
const FOUR_SPACE_SIZE: usize = 4;
const TWO_SPACE_SCORE_FACTOR: usize = 3;
const FOUR_SPACE_SCORE_FACTOR: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentOptions {
    pub tab_size: u32,
    pub insert_spaces: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelIndentOptions {
    pub tab_size: u32,
    pub indent_size: u32,
    pub insert_spaces: bool,
}

impl From<IndentOptions> for ModelIndentOptions {
    fn from(options: IndentOptions) -> Self {
        Self {
            tab_size: options.tab_size.max(1),
            indent_size: options.tab_size.max(1),
            insert_spaces: options.insert_spaces,
        }
    }
}

impl DocumentSnapshot {
    pub fn model_indentation(&self, fallback: IndentOptions) -> ModelIndentOptions {
        let options = self.indent_options.unwrap_or(fallback);
        ModelIndentOptions {
            tab_size: options.tab_size.max(1),
            indent_size: self.indent_size.unwrap_or(options.tab_size).max(1),
            insert_spaces: options.insert_spaces,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentationChange {
    UseSpaces(u32),
    UseTabs(u32),
    DisplaySize(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentConfiguration {
    pub defaults: IndentOptions,
    pub detect_indentation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    ToSpaces,
    ToTabs,
    Detect,
    ReindentLines,
    ReindentSelectedLines,
}

impl Command {
    pub fn from_action(action: &str) -> Option<Self> {
        match action {
            "editor.action.indentationToSpaces" => Some(Self::ToSpaces),
            "editor.action.indentationToTabs" => Some(Self::ToTabs),
            "editor.action.detectIndentation" => Some(Self::Detect),
            "editor.action.reindentlines" => Some(Self::ReindentLines),
            "editor.action.reindentselectedlines" => Some(Self::ReindentSelectedLines),
            _ => None,
        }
    }

    pub fn requires_write(self) -> bool {
        self != Self::Detect
    }
}

pub fn run_command(
    store: &mut EditorStore,
    view: ViewId,
    command: Command,
    configuration: IndentConfiguration,
) -> Result<bool, EditorError> {
    run_command_with_language(store, view, command, configuration, None)
}

pub fn run_command_with_language(
    store: &mut EditorStore,
    view: ViewId,
    command: Command,
    configuration: IndentConfiguration,
    language: Option<Language<'_>>,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    if command.requires_write() && document.metadata.read_only {
        return Err(EditorError::ReadOnly);
    }
    let options = store.configure_indentation(document.id, configuration)?;
    let document = store.documents().snapshot(document.id)?;
    if matches!(
        command,
        Command::ReindentLines | Command::ReindentSelectedLines
    ) {
        let Some(language) = language else {
            return Ok(false);
        };
        if language.rules.indent_metadata("").is_none() {
            return Ok(false);
        }
        language.syntax.follow_edits(store);
        let ranges = if command == Command::ReindentLines {
            vec![0..document.rope.len_lines()]
        } else {
            current
                .selection
                .selections
                .iter()
                .map(|selection| {
                    let bytes = ordered(selection);
                    let first = document.rope.byte_to_line(bytes.start);
                    let mut last = document.rope.byte_to_line(bytes.end);
                    if first < last && document.rope.line_to_byte(last) == bytes.end {
                        last -= 1;
                    }
                    first.saturating_sub(1)..last + 1
                })
                .collect()
        };
        let required = ranges.iter().map(|range| range.start).min().unwrap_or(0)
            ..ranges.iter().map(|range| range.end).max().unwrap_or(0);
        let prepared = language.syntax.prepare_tokens(&document, required)?;
        let language = Language {
            rules: language.rules,
            syntax: prepared.as_ref().map_or(language.syntax, |tokens| tokens),
        };
        let mut plan = Plan::new(&current.selection);
        for range in ranges {
            for edit in crate::auto_indent::reindent_edits(
                &document,
                language,
                document.model_indentation(options),
                range,
                store.document_byte_limit(),
            )? {
                plan.edit(edit.bytes, edit.text);
            }
        }
        if !plan.has_edits() {
            return Ok(false);
        }
        store.set_composition(view, None)?;
        let changed = apply_step(
            store,
            view,
            plan.transaction(&document, view),
            SEPARATE_STEP,
        )?;
        language.syntax.follow_edits(store);
        if changed {
            let selected = &store
                .views()
                .get(view)
                .ok_or(EditorError::NotFound)?
                .selection;
            let head = selected.selections[selected.primary].head;
            store.request_selection_reveal(view, head..head, false)?;
        }
        return Ok(changed);
    }
    if command == Command::Detect {
        let guessed = guess(&document.rope, configuration.defaults);
        let change = if guessed.insert_spaces {
            IndentationChange::UseSpaces(guessed.tab_size)
        } else {
            IndentationChange::UseTabs(guessed.tab_size)
        };
        return store.set_indentation(document.id, configuration, change);
    }
    let insert_spaces = command == Command::ToSpaces;
    let width = options.tab_size as usize;
    let mut projected = document.rope.len_bytes();
    let mut plan = Plan::new(&SelectionSet {
        primary: 0,
        selections: vec![current.selection.selections[current.selection.primary]],
    });
    for line in 0..document.rope.len_lines() {
        let slice = document.rope.line(line);
        let length = slice
            .chars()
            .take_while(|character| matches!(character, ' ' | '\t'))
            .count();
        if length == 0 {
            continue;
        }
        let prefix = slice.slice(..length).to_string();
        let tabs = prefix.bytes().filter(|byte| *byte == b'\t').count();
        let expanded = if insert_spaces {
            tabs.checked_mul(width)
                .and_then(|expanded_tabs| expanded_tabs.checked_add(length - tabs))
                .ok_or(EditorError::Capacity)?
        } else {
            length
        };
        projected = projected
            .checked_sub(length)
            .and_then(|bytes| bytes.checked_add(expanded))
            .ok_or(EditorError::Capacity)?;
        if projected > store.document_byte_limit() {
            return Err(EditorError::Capacity);
        }
        let converted = if insert_spaces && tabs > 0 {
            prefix.replace('\t', &" ".repeat(width))
        } else if insert_spaces {
            prefix.clone()
        } else {
            let mut converted = String::with_capacity(prefix.len());
            for (index, spaces) in prefix.split('\t').enumerate() {
                if index > 0 {
                    converted.push('\t');
                }
                converted.extend(std::iter::repeat_n('\t', spaces.len() / width));
                converted.extend(std::iter::repeat_n(' ', spaces.len() % width));
            }
            converted
        };
        if prefix != converted {
            let start = document.rope.line_to_byte(line);
            plan.edit(start..start + length, converted);
        }
    }
    store.set_composition(view, None)?;
    let changed = apply_step(
        store,
        view,
        plan.transaction(&document, view),
        SEPARATE_STEP,
    )?;
    let options_changed = store.override_indentation(
        document.id,
        configuration,
        IndentOptions {
            insert_spaces,
            ..options
        },
    )?;
    if changed {
        let selected = &store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .selection;
        let head = selected.selections[selected.primary].head;
        store.request_selection_reveal(view, head..head, false)?;
    }
    Ok(changed || options_changed)
}

fn is_space_at(text: RopeSlice<'_>, column: usize) -> bool {
    let mut offset = 0;
    for character in text.chars() {
        if offset >= column {
            return offset == column && character == ' ';
        }
        offset += character.len_utf16();
    }
    false
}

fn spaces_diff(
    previous: RopeSlice<'_>,
    previous_indent: usize,
    current: RopeSlice<'_>,
    current_indent: usize,
) -> (usize, bool) {
    let common = previous
        .chars()
        .zip(current.chars())
        .take(previous_indent.min(current_indent))
        .take_while(|(left, right)| left == right)
        .count();
    let previous_spaces = previous
        .chars()
        .skip(common)
        .take(previous_indent - common)
        .filter(|character| *character == ' ')
        .count();
    let current_spaces = current
        .chars()
        .skip(common)
        .take(current_indent - common)
        .filter(|character| *character == ' ')
        .count();
    let previous_tabs = previous_indent - common - previous_spaces;
    let current_tabs = current_indent - common - current_spaces;
    if (previous_spaces > 0 && previous_tabs > 0) || (current_spaces > 0 && current_tabs > 0) {
        return (0, false);
    }
    let tabs = previous_tabs.abs_diff(current_tabs);
    let spaces = previous_spaces.abs_diff(current_spaces);
    if tabs > 0 {
        return (
            if spaces.is_multiple_of(tabs) {
                spaces / tabs
            } else {
                0
            },
            false,
        );
    }
    let alignment = spaces > 0
        && current_spaces > 0
        && is_space_at(previous, current_spaces - 1)
        && !is_space_at(current, current_spaces)
        && previous
            .len_chars()
            .checked_sub(1)
            .is_some_and(|index| previous.char(index) == ',');
    (spaces, alignment)
}

pub fn guess(rope: &Rope, defaults: IndentOptions) -> IndentOptions {
    let mut tab_lines = 0;
    let mut space_lines = 0;
    let mut scores = [0; MAX_TAB_SIZE_GUESS + 1];
    let mut previous = rope.slice(..0);
    let mut previous_indent = 0;
    for line in 0..rope.len_lines().min(DETECTION_LINE_LIMIT) {
        let range = crate::editing::rope_line_content_range(rope, line);
        let current = rope.slice(rope.byte_to_char(range.start)..rope.byte_to_char(range.end));
        let mut spaces = 0;
        let mut tabs = 0;
        for character in current.chars() {
            match character {
                ' ' => spaces += 1,
                '\t' => tabs += 1,
                _ => break,
            }
        }
        let indentation = spaces + tabs;
        if indentation == current.len_chars() {
            continue;
        }
        if tabs > 0 {
            tab_lines += 1;
        } else if spaces > 1 {
            space_lines += 1;
        }
        let (difference, alignment) = spaces_diff(previous, previous_indent, current, indentation);
        if alignment && !(defaults.insert_spaces && defaults.tab_size as usize == difference) {
            continue;
        }
        if let Some(score) = scores.get_mut(difference) {
            *score += 1;
        }
        previous = current;
        previous_indent = indentation;
    }
    let insert_spaces = if tab_lines == space_lines {
        defaults.insert_spaces
    } else {
        tab_lines < space_lines
    };
    let mut tab_size = defaults.tab_size.max(1);
    if insert_spaces {
        let mut best_score = 0;
        for size in TAB_SIZE_GUESSES {
            if scores[size] > best_score {
                best_score = scores[size];
                tab_size = size as u32;
            }
        }
        if tab_size as usize == FOUR_SPACE_SIZE
            && scores[FOUR_SPACE_SIZE] > 0
            && scores[TWO_SPACE_SIZE] > 0
            && scores[TWO_SPACE_SIZE] * TWO_SPACE_SCORE_FACTOR
                >= scores[FOUR_SPACE_SIZE] * FOUR_SPACE_SCORE_FACTOR
        {
            tab_size = TWO_SPACE_SIZE as u32;
        }
    }
    IndentOptions {
        tab_size,
        insert_spaces,
    }
}

pub fn resolve(config: &EditorConfigOptions, current: IndentOptions) -> IndentOptions {
    let tab_size = match config.indent_style {
        Some(EditorConfigIndentStyle::Tab) => config.tab_width.or(config.indent_size),
        _ => config.indent_size.or(config.tab_width),
    }
    .unwrap_or(current.tab_size);
    let insert_spaces = config
        .indent_style
        .map(|style| style == EditorConfigIndentStyle::Space)
        .unwrap_or(current.insert_spaces);
    IndentOptions {
        tab_size,
        insert_spaces,
    }
}
