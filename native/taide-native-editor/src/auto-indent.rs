use std::ops::Range;

use crate::document::DocumentSnapshot;
use crate::editing::{
    LineText, indentation, leading_whitespace, line_text, next_tab_stop, previous_tab_stop,
    tab_width, visible_column,
};
use crate::indent::IndentOptions;
use crate::language_configuration::{
    IndentAction, IndentMetadata, Language, LanguageRules, is_js_whitespace, token_kind_at,
    without_brackets_outside_code,
};
use crate::syntax::{Token, TokenKind};

const INDENT_UNIT: &str = "\t";
const MAX_BRACKET_TREE_UTF16_LENGTH: usize = 5_000_000;

pub(crate) struct LineBreak {
    pub(crate) bytes: Range<usize>,
    pub(crate) text: String,
    pub(crate) caret_before_end: usize,
}

struct EnterContext {
    previous_line: String,
    before: String,
    after: String,
}

struct Inherited {
    indentation: String,
    indents: bool,
    line: Option<usize>,
}

struct Lines<'a> {
    document: &'a DocumentSnapshot,
    language: Language<'a>,
    replaced: Option<(usize, &'a str)>,
}

impl Lines<'_> {
    fn replacement(&self, line: usize) -> Option<&str> {
        self.replaced
            .filter(|(replaced, _)| *replaced == line)
            .map(|(_, text)| text)
    }

    fn content(&self, line: usize) -> String {
        match self.replacement(line) {
            Some(text) => text.into(),
            None => line_text(self.document, line).text,
        }
    }

    fn metadata(&self, line: usize) -> IndentMetadata {
        let processed = match self.replacement(line) {
            Some(text) => text.into(),
            None => processed_line(self.document, self.language, line),
        };
        self.language
            .rules
            .indent_metadata(&processed)
            .unwrap_or_default()
    }

    fn inherited(&self, line: usize, indents: bool) -> Inherited {
        Inherited {
            indentation: leading_whitespace(&self.content(line)).into(),
            indents,
            line: Some(line),
        }
    }
}

fn normalized(text: &str, indent: IndentOptions) -> String {
    let whitespace = leading_whitespace(text);
    format!(
        "{}{}",
        indentation(visible_column(whitespace, tab_width(indent)), indent),
        &text[whitespace.len()..]
    )
}

fn shifted(indent_text: &str, indent: IndentOptions) -> String {
    let size = tab_width(indent);
    indentation(
        next_tab_stop(visible_column(indent_text, size), size),
        indent,
    )
}

fn unshifted(indent_text: &str, indent: IndentOptions) -> String {
    let size = tab_width(indent);
    indentation(
        previous_tab_stop(visible_column(indent_text, size), size),
        indent,
    )
}

fn code_tokens(text: &str) -> Vec<Token> {
    if text.is_empty() {
        return Vec::new();
    }
    vec![Token {
        start_byte: 0,
        kind: TokenKind::Other,
    }]
}

fn line_tokens(
    document: &DocumentSnapshot,
    language: Language<'_>,
    line: usize,
    text: &str,
) -> Vec<Token> {
    language
        .syntax
        .tokens(document, line)
        .unwrap_or_else(|| code_tokens(text))
}

fn processed_line(document: &DocumentSnapshot, language: Language<'_>, line: usize) -> String {
    let content = line_text(document, line);
    let tokens = line_tokens(document, language, line, &content.text);
    without_brackets_outside_code(
        language.rules,
        &content.text,
        &tokens,
        0..content.text.len(),
    )
}

fn enter_context(
    document: &DocumentSnapshot,
    range: &Range<usize>,
    language: Language<'_>,
    start: &LineText,
    start_tokens: &[Token],
) -> EnterContext {
    let start_line = document.rope.byte_to_line(range.start);
    let column = (range.start - start.start).min(start.text.len());
    let end_line = document.rope.byte_to_line(range.end);
    let end = line_text(document, end_line);
    let end_tokens = line_tokens(document, language, end_line, &end.text);
    let end_column = (range.end - end.start).min(end.text.len());
    EnterContext {
        previous_line: start_line
            .checked_sub(1)
            .map(|previous| processed_line(document, language, previous))
            .unwrap_or_default(),
        before: without_brackets_outside_code(language.rules, &start.text, start_tokens, 0..column),
        after: without_brackets_outside_code(
            language.rules,
            &end.text,
            &end_tokens,
            end_column..end.text.len(),
        ),
    }
}

fn preceding_valid_line(lines: &Lines<'_>, line: usize) -> Option<usize> {
    (0..line).rev().find(|candidate| {
        let text = lines.content(*candidate);
        !(lines.metadata(*candidate).is_unindented || text.chars().all(is_js_whitespace))
    })
}

fn inherited_indent(
    lines: &Lines<'_>,
    line: usize,
    honors_intentional_indent: bool,
) -> Option<Inherited> {
    let unindented = || Inherited {
        indentation: String::new(),
        indents: false,
        line: None,
    };
    if (0..line).rev().all(|prior| lines.content(prior).is_empty()) {
        return Some(unindented());
    }
    let preceding = preceding_valid_line(lines, line)?;
    let metadata = lines.metadata(preceding);
    if metadata.increases || metadata.indents_next_line {
        return Some(lines.inherited(preceding, true));
    }
    if metadata.decreases || preceding == 0 {
        return Some(lines.inherited(preceding, false));
    }
    let previous = preceding - 1;
    let previous_metadata = lines
        .language
        .rules
        .indent_metadata(&lines.content(previous))
        .unwrap_or_default();
    if !previous_metadata.increases
        && !previous_metadata.decreases
        && previous_metadata.indents_next_line
    {
        let settled = (0..previous)
            .rev()
            .find(|candidate| !lines.metadata(*candidate).indents_next_line)
            .map_or(0, |stop| stop + 1);
        return Some(lines.inherited(settled, false));
    }
    if honors_intentional_indent {
        return Some(lines.inherited(preceding, false));
    }
    for candidate in (0..=preceding).rev() {
        let metadata = lines.metadata(candidate);
        if metadata.increases {
            return Some(lines.inherited(candidate, true));
        }
        if metadata.indents_next_line {
            return Some(lines.inherited(0, false));
        }
        if metadata.decreases {
            return Some(lines.inherited(candidate, false));
        }
    }
    Some(lines.inherited(0, false))
}

fn decreases(rules: &dyn LanguageRules, text: &str) -> bool {
    rules
        .indent_metadata(text)
        .is_some_and(|metadata| metadata.decreases)
}

pub(crate) fn good_indent(
    document: &DocumentSnapshot,
    language: Language<'_>,
    line: usize,
    indent: IndentOptions,
) -> Option<String> {
    language.rules.indent_metadata("")?;
    let lines = Lines {
        document,
        language,
        replaced: None,
    };
    let inherited = inherited_indent(&lines, line, true)?;
    let current_decreases = lines.metadata(line).decreases;
    if let Some(origin) = inherited.line
        && (origin..line.saturating_sub(1))
            .all(|between| lines.content(between).chars().all(is_js_whitespace))
        && let Some(action) = language.rules.enter_action("", &lines.content(origin), "")
    {
        let mut indentation_text = leading_whitespace(&lines.content(origin)).to_string();
        if let Some(remove) = action.remove_text {
            indentation_text.truncate(indentation_text.len().saturating_sub(remove));
        }
        indentation_text = match action.indent_action {
            IndentAction::Indent | IndentAction::IndentOutdent => {
                shifted(&indentation_text, indent)
            }
            IndentAction::Outdent => unshifted(&indentation_text, indent),
            IndentAction::None => indentation_text,
        };
        if current_decreases {
            indentation_text = unshifted(&indentation_text, indent);
        }
        indentation_text.push_str(action.append_text.as_deref().unwrap_or_default());
        return Some(leading_whitespace(&indentation_text).into());
    }
    Some(match (inherited.indents, current_decreases) {
        (true, false) => shifted(&inherited.indentation, indent),
        (false, true) => unshifted(&inherited.indentation, indent),
        _ => inherited.indentation,
    })
}

pub(crate) fn enter_prefix(
    document: &DocumentSnapshot,
    language: Language<'_>,
    line: usize,
    indent: IndentOptions,
) -> Option<String> {
    let before = processed_line(document, language, line);
    let previous = line
        .checked_sub(1)
        .map(|line| processed_line(document, language, line))
        .unwrap_or_default();
    let action = language.rules.enter_action(&previous, &before, "")?;
    let mut prefix = leading_whitespace(&line_text(document, line).text).to_string();
    if let Some(remove) = action.remove_text {
        prefix.truncate(prefix.len().saturating_sub(remove));
    }
    if action.indent_action == IndentAction::Outdent {
        prefix = unshifted(&prefix, indent);
    }
    let append = action.append_text.as_deref().unwrap_or("");
    match action.indent_action {
        IndentAction::Indent if append.is_empty() => prefix.push_str(INDENT_UNIT),
        IndentAction::Indent => {
            prefix.push_str(INDENT_UNIT);
            prefix.push_str(append);
        }
        IndentAction::IndentOutdent => {}
        _ => prefix.push_str(append),
    }
    Some(prefix)
}

pub(crate) fn extra_indent_spaces(
    document: &DocumentSnapshot,
    language: Language<'_>,
    line: usize,
    previous: usize,
    size: usize,
) -> Option<usize> {
    let before = processed_line(document, language, line);
    let prior = line
        .checked_sub(1)
        .map(|line| processed_line(document, language, line))
        .unwrap_or_default();
    let action = language.rules.enter_action(&prior, &before, "")?;
    let append = if action.indent_action == IndentAction::Indent {
        "\t"
    } else {
        action.append_text.as_deref().unwrap_or_default()
    };
    let spaces = append
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    Some(
        (previous + spaces)
            .min(size)
            .saturating_sub(action.remove_text.unwrap_or(0)),
    )
}

fn indent_for_enter(
    document: &DocumentSnapshot,
    start_line: usize,
    language: Language<'_>,
    context: &EnterContext,
    indent: IndentOptions,
) -> Option<String> {
    language.rules.indent_metadata("")?;
    let lines = Lines {
        document,
        language,
        replaced: Some((start_line, &context.before)),
    };
    let Some(inherited) = inherited_indent(&lines, start_line + 1, true) else {
        return Some(leading_whitespace(&context.before).into());
    };
    let mut after_enter = inherited.indentation;
    if inherited.indents {
        after_enter = shifted(&after_enter, indent);
    }
    if decreases(language.rules, &context.after) {
        after_enter = unshifted(&after_enter, indent);
    }
    Some(after_enter)
}

pub(crate) fn line_break(
    document: &DocumentSnapshot,
    range: Range<usize>,
    language: Language<'_>,
    indent: IndentOptions,
    line_ending: &str,
) -> LineBreak {
    let start_line = document.rope.byte_to_line(range.start);
    let start = line_text(document, start_line);
    let column = (range.start - start.start).min(start.text.len());
    let leading = leading_whitespace(&start.text);
    let kept = &leading[..leading.len().min(column)];
    let plain = |bytes: Range<usize>, indentation: String| LineBreak {
        bytes,
        text: format!("{line_ending}{indentation}"),
        caret_before_end: 0,
    };
    let Some(tokens) = language.syntax.tokens(document, start_line) else {
        return plain(range, normalized(kept, indent));
    };
    let context = enter_context(document, &range, language, &start, &tokens);
    if let Some(action) =
        language
            .rules
            .enter_action(&context.previous_line, &context.before, &context.after)
    {
        let indents = matches!(
            action.indent_action,
            IndentAction::Indent | IndentAction::IndentOutdent
        );
        let appended = match action
            .append_text
            .as_deref()
            .filter(|text| !text.is_empty())
        {
            None if indents => INDENT_UNIT.into(),
            None => String::new(),
            Some(text) if action.indent_action == IndentAction::Indent => {
                format!("{INDENT_UNIT}{text}")
            }
            Some(text) => text.into(),
        };
        let kept = &kept[..kept
            .len()
            .saturating_sub(action.remove_text.unwrap_or_default())];
        let indented = || normalized(&format!("{kept}{appended}"), indent);
        return match action.indent_action {
            IndentAction::None | IndentAction::Indent => plain(range, indented()),
            IndentAction::IndentOutdent => {
                let normal = normalized(kept, indent);
                LineBreak {
                    bytes: range,
                    caret_before_end: line_ending.len() + normal.len(),
                    text: format!("{line_ending}{}{line_ending}{normal}", indented()),
                }
            }
            IndentAction::Outdent => plain(
                range,
                normalized(&format!("{}{appended}", unshifted(kept, indent)), indent),
            ),
        };
    }
    let Some(after_enter) = indent_for_enter(document, start_line, language, &context, indent)
    else {
        return plain(range, normalized(kept, indent));
    };
    let end = line_text(document, document.rope.byte_to_line(range.end));
    let end_column = (range.end - end.start).min(end.text.len());
    let first_content = leading_whitespace(&end.text).len();
    let has_content = first_content < end.text.len();
    let swallowed_end = if has_content {
        end_column.max(first_content)
    } else {
        end.text.len()
    };
    let indentation = normalized(&after_enter, indent);
    let caret_before_end = if has_content && end_column <= first_content {
        let size = tab_width(indent);
        let end_columns = visible_column(&end.text[..end_column], size);
        let end_stops = if indent.insert_spaces {
            end_columns
        } else {
            end_columns.div_ceil(size)
        };
        indentation.len().saturating_sub(end_stops)
    } else {
        0
    };
    LineBreak {
        bytes: range.start..end.start + swallowed_end,
        text: format!("{line_ending}{indentation}"),
        caret_before_end,
    }
}

pub(crate) fn typed_reindent(
    document: &DocumentSnapshot,
    range: &Range<usize>,
    character: char,
    language: Language<'_>,
    indent: IndentOptions,
) -> Option<(Range<usize>, String)> {
    let rules = language.rules;
    rules.indent_metadata("")?;
    language
        .syntax
        .tokens(document, document.rope.byte_to_line(range.end))?;
    let start_line = document.rope.byte_to_line(range.start);
    let start = line_text(document, start_line);
    let start_tokens = line_tokens(document, language, start_line, &start.text);
    let context = enter_context(document, range, language, &start, &start_tokens);
    let around = format!("{}{}", context.before, context.after);
    let typed = format!("{}{character}{}", context.before, context.after);
    let lines = Lines {
        document,
        language,
        replaced: None,
    };
    let actual = if !decreases(rules, &around) && decreases(rules, &typed) {
        let inherited = inherited_indent(&lines, start_line, false)?;
        if inherited.indents {
            inherited.indentation
        } else {
            unshifted(&inherited.indentation, indent)
        }
    } else {
        let previous = line_text(document, start_line.checked_sub(1)?);
        if !rules.indent_metadata(&previous.text)?.indents_next_line
            || !rules.indent_metadata(&typed)?.increases
        {
            return None;
        }
        let inherited = inherited_indent(&lines, start_line, false)?.indentation;
        let is_inferred = shifted(&inherited, indent) == leading_whitespace(&start.text);
        let opens_pair = rules
            .pairs()
            .auto_closing_pairs
            .iter()
            .any(|pair| pair.open.ends_with(character));
        if !is_inferred || !opens_pair || !around.chars().all(is_js_whitespace) {
            return None;
        }
        inherited
    };
    let column = (range.start - start.start).min(start.text.len());
    let leading = leading_whitespace(&start.text).len();
    if actual == normalized(&start.text[..leading.min(column)], indent) {
        return None;
    }
    let mut text = normalized(&actual, indent);
    if leading < start.text.len() {
        text.push_str(&start.text[leading.min(column)..leading.max(column)]);
    }
    text.push(character);
    Some((start.start..range.end, text))
}

fn enclosing_opener_line(
    document: &DocumentSnapshot,
    line: usize,
    column: usize,
    closer: &str,
    language: Language<'_>,
) -> Option<usize> {
    let rules = language.rules;
    let pairs = rules.pairs();
    if !pairs.is_closing_bracket(closer)
        || document.rope.len_utf16_cu() > MAX_BRACKET_TREE_UTF16_LENGTH
    {
        return None;
    }
    let mut open: Vec<(String, usize)> = Vec::new();
    for current in 0..=line {
        let content = line_text(document, current);
        let tokens = language.syntax.tokens(document, current)?;
        for range in rules.bracket_ranges(&content.text) {
            if current == line && range.end > column {
                break;
            }
            if token_kind_at(&tokens, range.start) != TokenKind::Other {
                continue;
            }
            let text = content.text[range].to_lowercase();
            if pairs.is_closing_bracket(&text) {
                if let Some(index) = open
                    .iter()
                    .rposition(|(opener, _)| pairs.closes(&text, opener))
                {
                    open.truncate(index);
                }
            } else if pairs.brackets.iter().any(|pair| pair.open == text) {
                open.push((text, current));
            }
        }
    }
    open.iter()
        .rev()
        .find(|(opener, _)| pairs.closes(closer, opener))
        .map(|(_, line)| *line)
}

pub(crate) fn electric_reindent(
    document: &DocumentSnapshot,
    head: usize,
    character: char,
    language: Language<'_>,
    indent: IndentOptions,
) -> Option<(Range<usize>, String)> {
    let pairs = language.rules.pairs();
    let is_electric = pairs
        .brackets
        .iter()
        .any(|pair| pair.close.to_lowercase().ends_with(character));
    if !is_electric {
        return None;
    }
    let line = document.rope.byte_to_line(head);
    let content = line_text(document, line);
    let column = (head - content.start).min(content.text.len());
    let tokens = language.syntax.tokens(document, line)?;
    if token_kind_at(&tokens, column) != TokenKind::Other {
        return None;
    }
    let typed = format!("{}{character}", &content.text[..column]);
    let found = language.rules.last_bracket(&typed)?;
    let bracket = typed[found.clone()].to_lowercase();
    if pairs.is_opening_bracket(&bracket)
        || !content.text[..found.start.min(column)]
            .chars()
            .all(is_js_whitespace)
    {
        return None;
    }
    let bracket_column = format!("{}{character}", content.text)
        .rfind(&bracket)
        .unwrap_or_default();
    let opener_line = enclosing_opener_line(document, line, bracket_column, &bracket, language)?;
    if opener_line == line {
        return None;
    }
    let opener = line_text(document, opener_line);
    let leading = leading_whitespace(&content.text).len();
    let first_content = if leading < content.text.len() {
        leading
    } else {
        column
    };
    Some((
        content.start..head,
        format!(
            "{}{}{character}",
            normalized(leading_whitespace(&opener.text), indent),
            &content.text[first_content.min(column)..first_content.max(column)]
        ),
    ))
}
