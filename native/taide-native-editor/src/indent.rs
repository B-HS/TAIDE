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
pub struct IndentConfiguration {
    pub defaults: IndentOptions,
    pub detect_indentation: bool,
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
