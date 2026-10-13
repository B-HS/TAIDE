use crate::document::EditorError;

use super::{ExpandedMarkupNode, MarkupValue};

#[path = "emmet-markup-lorem-defaults.rs"]
mod defaults;

const DEFAULT_WORD_COUNT: usize = 30;
const SENTENCE_MIN_WORDS: usize = 2;
const SENTENCE_MAX_WORDS: usize = 30;
const SHORT_COMMA_MIN_WORDS: usize = 3;
const SHORT_COMMA_MAX_WORDS: usize = 6;
const MEDIUM_COMMA_MAX_WORDS: usize = 12;
const LONG_MIN_COMMAS: usize = 1;
const LONG_MAX_COMMAS: usize = 4;
const PUNCTUATION: &[&str] = &["?", "!", ".", ".", "."];

pub(crate) struct Lorem<'a> {
    pub random: &'a mut dyn FnMut() -> u32,
    pub remaining_draws: usize,
    pub max_bytes: usize,
}

impl Lorem<'_> {
    fn draw(&mut self, from: usize, to: usize) -> Result<usize, EditorError> {
        self.remaining_draws = self
            .remaining_draws
            .checked_sub(1)
            .ok_or(EditorError::Capacity)?;
        let offset = (u128::from((self.random)()) * (to - from) as u128) >> u32::BITS;
        Ok(from + offset as usize)
    }

    fn sentence(&mut self, mut words: Vec<&str>, is_common: bool) -> Result<String, EditorError> {
        let length = words.len();
        let mut commas = Vec::new();
        if length >= SENTENCE_MIN_WORDS {
            let total = match length {
                length if length > SHORT_COMMA_MIN_WORDS && length <= SHORT_COMMA_MAX_WORDS => {
                    self.draw(0, 1)?
                }
                length if length > SHORT_COMMA_MAX_WORDS && length <= MEDIUM_COMMA_MAX_WORDS => {
                    self.draw(0, SENTENCE_MIN_WORDS)?
                }
                _ => self.draw(LONG_MIN_COMMAS, LONG_MAX_COMMAS)?,
            };
            for _ in 0..total {
                let position = self.draw(0, length - SENTENCE_MIN_WORDS)?;
                if !commas.contains(&position) {
                    commas.push(position);
                }
            }
        }
        let mut sentence = String::new();
        for (index, word) in words.drain(..).enumerate() {
            if index > 0 {
                sentence.push(' ');
            }
            if index == 0 {
                let mut characters = word.chars();
                if let Some(first) = characters.next() {
                    sentence.extend(first.to_uppercase());
                }
                sentence.push_str(characters.as_str());
            } else {
                sentence.push_str(word);
            }
            if commas.contains(&index) {
                sentence.push(',');
            }
            if sentence.len() > self.max_bytes {
                return Err(EditorError::Capacity);
            }
        }
        let punctuation = if is_common {
            "."
        } else {
            PUNCTUATION[self.draw(0, PUNCTUATION.len() - 1)?]
        };
        sentence.push_str(punctuation);
        Ok(sentence)
    }

    pub fn resolve(
        &mut self,
        node: &mut ExpandedMarkupNode,
        inherited_repeat: Option<usize>,
    ) -> Result<bool, EditorError> {
        let Some(name) = node.name.as_deref() else {
            return Ok(false);
        };
        let Some(prefix) = name.get(.."lorem".len()) else {
            return Ok(false);
        };
        if !prefix.eq_ignore_ascii_case("lorem") {
            return Ok(false);
        }
        let suffix = &name["lorem".len()..];
        let language_end = suffix
            .find(|character: char| !character.is_ascii_alphabetic())
            .unwrap_or(suffix.len());
        let (language, count) = suffix.split_at(language_end);
        let (min, max) = count.split_once('-').unwrap_or((count, ""));
        if !min.bytes().all(|byte| byte.is_ascii_digit())
            || !max.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Ok(false);
        }
        let min = if min.is_empty() {
            DEFAULT_WORD_COUNT
        } else {
            min.parse::<usize>()
                .map_err(|_| EditorError::Capacity)?
                .max(1)
        };
        let max = if max.is_empty() {
            min
        } else {
            max.parse::<usize>()
                .map_err(|_| EditorError::Capacity)?
                .max(min)
        };
        let word_count = self.draw(min, max)?;
        if word_count > self.max_bytes {
            return Err(EditorError::Capacity);
        }
        let (common, vocabulary) = match language {
            "ru" => (defaults::RU_COMMON, defaults::RU_WORDS),
            "sp" => (defaults::SP_COMMON, defaults::SP_WORDS),
            _ => (defaults::LATIN_COMMON, defaults::LATIN_WORDS),
        };
        let repeat = node.repeat.map(|repeat| repeat.index).or(inherited_repeat);
        let mut paragraph = String::new();
        let mut total_words = 0;
        if repeat.is_none_or(|index| index == 0) {
            let words = common[..common.len().min(word_count)].to_vec();
            total_words += words.len();
            paragraph = self.sentence(words, true)?;
        }
        while total_words < word_count {
            let count = self
                .draw(SENTENCE_MIN_WORDS, SENTENCE_MAX_WORDS)?
                .min(word_count - total_words)
                .min(vocabulary.len());
            let mut words = Vec::new();
            while words.len() < count {
                let word = vocabulary[self.draw(0, vocabulary.len())?];
                if !words.contains(&word) {
                    words.push(word);
                }
            }
            total_words += words.len();
            let sentence = self.sentence(words, false)?;
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            if paragraph.len().saturating_add(sentence.len()) > self.max_bytes {
                return Err(EditorError::Capacity);
            }
            paragraph.push_str(&sentence);
        }
        node.name = None;
        node.attributes = None;
        node.value = Some(vec![MarkupValue::Text(paragraph)]);
        Ok(true)
    }
}
