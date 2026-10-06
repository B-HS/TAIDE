use std::collections::HashMap;
use std::sync::LazyLock;

const BREAK_BEFORE_CHARACTERS: &str = "([{‘“〈《「『【〔（［｛｢£¥＄￡￥+＋";
const BREAK_AFTER_CHARACTERS: &str = " \t})]?|/&.,;¢°′″‰℃、。｡､￠，．：；？！％・･ゝゞヽヾーァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎゕゖㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ々〻ｧｨｩｪｫｬｭｮｯｰ”〉》」』】〕）］｝｣";
const ASCII_MAP_SIZE: usize = 256;
const FIRST_PRINTABLE_CODE: u32 = 32;
const FIRST_SUPPLEMENTARY_CODE: u32 = 0x10000;
const SUPPLEMENTARY_COLUMNS: f64 = 2.0;
const FULL_WIDTH_UTF8_BYTES: f64 = 3.0;
const INDENT_TABS: u32 = 1;
const DEEP_INDENT_TABS: u32 = 2;
const IDEOGRAPHIC_RANGES: [(u32, u32); 3] = [(0x3040, 0x30FF), (0x3400, 0x4DBF), (0x4E00, 0x9FFF)];
const FULL_WIDTH_RANGES: [(u32, u32); 4] = [
    (0x2E80, 0xD7AF),
    (0xF900, 0xFAFF),
    (0xFF01, 0xFF5E),
    (0xFFE0, 0xFFE6),
];

static CLASSIFIER: LazyLock<WrappingCharacterClassifier> = LazyLock::new(|| {
    WrappingCharacterClassifier::new(BREAK_BEFORE_CHARACTERS, BREAK_AFTER_CHARACTERS)
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrappingIndent {
    None,
    Same,
    Indent,
    DeepIndent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WrapSettings {
    pub wrap_column: u32,
    pub tab_size: u32,
    pub full_width_columns: f64,
    pub wrapping_indent: WrappingIndent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LineBreakData {
    pub break_offsets: Vec<usize>,
    pub break_offsets_visible_column: Vec<f64>,
    pub wrapped_text_indent_length: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharacterClass {
    None,
    BreakBefore,
    BreakAfter,
    BreakIdeographic,
}

struct WrappingCharacterClassifier {
    ascii: [CharacterClass; ASCII_MAP_SIZE],
    others: HashMap<char, CharacterClass>,
}

impl WrappingCharacterClassifier {
    fn new(break_before: &str, break_after: &str) -> Self {
        let mut classifier = Self {
            ascii: [CharacterClass::None; ASCII_MAP_SIZE],
            others: HashMap::new(),
        };
        for (characters, class) in [
            (break_before, CharacterClass::BreakBefore),
            (break_after, CharacterClass::BreakAfter),
        ] {
            for character in characters.chars() {
                match classifier.ascii.get_mut(u32::from(character) as usize) {
                    Some(slot) => *slot = class,
                    None => {
                        classifier.others.insert(character, class);
                    }
                }
            }
        }
        classifier
    }

    fn get(&self, character: char) -> CharacterClass {
        let code = u32::from(character);
        if code >= FIRST_SUPPLEMENTARY_CODE {
            return CharacterClass::None;
        }
        if let Some(class) = self.ascii.get(code as usize) {
            return *class;
        }
        if IDEOGRAPHIC_RANGES
            .iter()
            .any(|(first, last)| (*first..=*last).contains(&code))
        {
            return CharacterClass::BreakIdeographic;
        }
        self.others
            .get(&character)
            .copied()
            .unwrap_or(CharacterClass::None)
    }
}

pub fn is_full_width_character(character: char) -> bool {
    let code = u32::from(character);
    FULL_WIDTH_RANGES
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&code))
}

pub fn tab_columns(visible_column: f64, tab_size: u32) -> f64 {
    let tab_size = f64::from(tab_size.max(1));
    tab_size - visible_column % tab_size
}

fn character_columns(character: char, visible_column: f64, settings: &WrapSettings) -> f64 {
    if character == '\t' {
        return tab_columns(visible_column, settings.tab_size);
    }
    if character.is_ascii() {
        return if u32::from(character) < FIRST_PRINTABLE_CODE {
            settings.full_width_columns
        } else {
            1.0
        };
    }
    if u32::from(character) >= FIRST_SUPPLEMENTARY_CODE {
        return SUPPLEMENTARY_COLUMNS;
    }
    if is_full_width_character(character) {
        settings.full_width_columns
    } else {
        1.0
    }
}

fn can_break(previous: CharacterClass, character: char, class: CharacterClass) -> bool {
    character != ' '
        && ((previous == CharacterClass::BreakAfter && class != CharacterClass::BreakAfter)
            || (previous != CharacterClass::BreakBefore && class == CharacterClass::BreakBefore)
            || (previous == CharacterClass::BreakIdeographic
                && class != CharacterClass::BreakAfter)
            || (class == CharacterClass::BreakIdeographic
                && previous != CharacterClass::BreakBefore))
}

fn wrapped_text_indent_length(settings: &WrapSettings, text: &str) -> u32 {
    let additional_tabs = match settings.wrapping_indent {
        WrappingIndent::None => return 0,
        WrappingIndent::Same => 0,
        WrappingIndent::Indent => INDENT_TABS,
        WrappingIndent::DeepIndent => DEEP_INDENT_TABS,
    };
    let Some(first_non_whitespace) = text.bytes().position(|byte| !matches!(byte, b' ' | b'\t'))
    else {
        return 0;
    };
    let tab_size = settings.tab_size.max(1);
    let next_tab_stop = |column: u32| column + tab_size - column % tab_size;
    let existing = text.as_bytes()[..first_non_whitespace]
        .iter()
        .fold(0, |column, byte| {
            if *byte == b'\t' {
                next_tab_stop(column)
            } else {
                column + 1
            }
        });
    let indent = (0..additional_tabs).fold(existing, |column, _| next_tab_stop(column));
    if f64::from(indent) + settings.full_width_columns > f64::from(settings.wrap_column) {
        0
    } else {
        indent
    }
}

fn fits_by_byte_length(settings: &WrapSettings, text: &str) -> bool {
    let wrap_column = settings.wrap_column as usize;
    if settings.full_width_columns > FULL_WIDTH_UTF8_BYTES || text.len() > wrap_column {
        return false;
    }
    let widest_tab = settings.tab_size.max(1) as usize;
    let mut widest_columns = text.len();
    for byte in text.bytes() {
        if byte == b'\t' {
            widest_columns += widest_tab - 1;
        } else if u32::from(byte) < FIRST_PRINTABLE_CODE {
            return false;
        }
    }
    widest_columns <= wrap_column
}

pub fn create_line_breaks(settings: &WrapSettings, text: &str) -> Option<LineBreakData> {
    if fits_by_byte_length(settings, text) {
        return None;
    }
    let classifier = &*CLASSIFIER;
    let wrapped_text_indent_length = wrapped_text_indent_length(settings, text);
    let wrapped_line_break_column =
        f64::from(settings.wrap_column) - f64::from(wrapped_text_indent_length);
    let mut characters = text.char_indices();
    let (_, first) = characters.next()?;
    let mut previous_class = classifier.get(first);
    let mut visible_column = character_columns(first, 0.0, settings);
    let mut breaking_column = f64::from(settings.wrap_column);
    let mut candidate: Option<(usize, f64)> = None;
    let mut break_offsets = Vec::new();
    let mut break_offsets_visible_column = Vec::new();
    for (offset, character) in characters {
        let class = classifier.get(character);
        let columns = character_columns(character, visible_column, settings);
        if can_break(previous_class, character, class) {
            candidate = Some((offset, visible_column));
        }
        visible_column += columns;
        if visible_column > breaking_column {
            let (break_offset, break_column) = candidate
                .take()
                .filter(|(_, column)| visible_column - column <= wrapped_line_break_column)
                .unwrap_or((offset, visible_column - columns));
            break_offsets.push(break_offset);
            break_offsets_visible_column.push(break_column);
            breaking_column = break_column + wrapped_line_break_column;
        }
        previous_class = class;
    }
    if break_offsets.is_empty() {
        return None;
    }
    break_offsets.push(text.len());
    break_offsets_visible_column.push(visible_column);
    Some(LineBreakData {
        break_offsets,
        break_offsets_visible_column,
        wrapped_text_indent_length,
    })
}
