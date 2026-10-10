use std::ops::Range;

const MAX_UNITS: usize = 128;
const TABLE_SIDE: usize = MAX_UNITS + 1;
const ANY_PATTERN_LIMIT: usize = 13;
const MIN_TYPO_UNITS: usize = 3;
const MAX_TYPO_UNITS: usize = 7;
const TYPO_PENALTY: i32 = 3;
const EXACT_BOUNDARY_SCORE: i32 = 7;
const FOLDED_BOUNDARY_SCORE: i32 = 5;
const GAP_PENALTY: i32 = 5;
const PREFERRED_GAP_PENALTY: i32 = 3;
const NEW_BOUNDARY_BONUS: i32 = 2;
const FULL_MATCH_BONUS: i32 = 2;
const LONG_BACKWARDS_MATCH: usize = 2;
const DIAGONAL: u8 = 1;
const LEFT: u8 = 2;
const LEFT_LEFT: u8 = 3;
const LEFT_LEFT_DISTANCE: usize = 2;
const SPACE: u16 = b' ' as u16;
const TAB: u16 = b'\t' as u16;
const HIGH_SURROGATE_START: u16 = 0xd800;
const HIGH_SURROGATE_END: u16 = 0xdbff;
const LOW_SURROGATE_START: u16 = 0xdc00;
const LOW_SURROGATE_END: u16 = 0xdfff;
const SURROGATE_BASE: u32 = 0x10000;
const SURROGATE_SHIFT: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Score {
    pub value: i32,
    pub word_start: usize,
    pub positions: Vec<usize>,
}

impl Score {
    pub fn highlights(&self) -> Vec<Range<usize>> {
        let mut ranges: Vec<Range<usize>> = Vec::new();
        for position in self.positions.iter().rev() {
            let position = position + self.word_start;
            if let Some(last) = ranges.last_mut()
                && last.end == position
            {
                last.end = position + 1;
                continue;
            }
            ranges.push(position..position + 1);
        }
        ranges
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScoreOptions {
    pub first_match_can_be_weak: bool,
    pub boost_full_match: bool,
}

impl Default for ScoreOptions {
    fn default() -> Self {
        Self {
            first_match_can_be_weak: false,
            boost_full_match: true,
        }
    }
}

struct Text {
    units: Vec<u16>,
    lower: Vec<u16>,
}

impl Text {
    fn new(value: &str) -> Self {
        Self {
            units: value.encode_utf16().collect(),
            lower: value.to_lowercase().encode_utf16().collect(),
        }
    }

    fn swapped(&self, index: usize) -> Self {
        let mut units = self.units.clone();
        units.swap(index, index + 1);
        let lower = match String::from_utf16(&units) {
            Ok(text) => text.to_lowercase().encode_utf16().collect(),
            Err(_) => {
                let mut lower = Vec::new();
                let mut valid = String::new();
                for character in char::decode_utf16(units.iter().copied()) {
                    match character {
                        Ok(character) => valid.push(character),
                        Err(error) => {
                            lower.extend(valid.to_lowercase().encode_utf16());
                            valid.clear();
                            lower.push(error.unpaired_surrogate());
                        }
                    }
                }
                lower.extend(valid.to_lowercase().encode_utf16());
                lower
            }
        };
        Self { units, lower }
    }

    fn is_uppercase(&self, position: usize) -> bool {
        self.units.get(position) != self.lower.get(position)
    }
}

pub struct Scorer {
    minimum: Vec<usize>,
    maximum: Vec<usize>,
    diagonals: Vec<usize>,
    scores: Vec<i32>,
    arrows: Vec<u8>,
}

impl Default for Scorer {
    fn default() -> Self {
        Self {
            minimum: vec![0; TABLE_SIDE],
            maximum: vec![0; TABLE_SIDE],
            diagonals: vec![0; TABLE_SIDE * TABLE_SIDE],
            scores: vec![0; TABLE_SIDE * TABLE_SIDE],
            arrows: vec![0; TABLE_SIDE * TABLE_SIDE],
        }
    }
}

impl Scorer {
    pub fn score(
        &mut self,
        pattern: &str,
        pattern_start: usize,
        word: &str,
        word_start: usize,
        options: ScoreOptions,
    ) -> Option<Score> {
        self.score_text(
            &Text::new(pattern),
            pattern_start,
            &Text::new(word),
            word_start,
            options,
        )
    }

    pub fn graceful(
        &mut self,
        pattern: &str,
        pattern_start: usize,
        word: &str,
        word_start: usize,
        options: ScoreOptions,
    ) -> Option<Score> {
        let pattern = Text::new(pattern);
        let word = Text::new(word);
        let mut best = self.score_text(&pattern, pattern_start, &word, word_start, options);
        if pattern.units.len() >= MIN_TYPO_UNITS {
            let tries = MAX_TYPO_UNITS.min(pattern.units.len() - 1);
            for position in pattern_start.saturating_add(1)..tries {
                if pattern.units[position] == pattern.units[position + 1] {
                    continue;
                }
                if let Some(mut candidate) = self.score_text(
                    &pattern.swapped(position),
                    pattern_start,
                    &word,
                    word_start,
                    options,
                ) {
                    candidate.value -= TYPO_PENALTY;
                    if best
                        .as_ref()
                        .is_none_or(|best| candidate.value > best.value)
                    {
                        best = Some(candidate);
                    }
                }
            }
        }
        best
    }

    pub fn any(
        &mut self,
        pattern: &str,
        pattern_start: usize,
        word: &str,
        word_start: usize,
    ) -> Score {
        let pattern = Text::new(pattern);
        let word = Text::new(word);
        for start in pattern_start..ANY_PATTERN_LIMIT.min(pattern.units.len()) {
            if let Some(score) = self.score_text(
                &pattern,
                start,
                &word,
                word_start,
                ScoreOptions {
                    first_match_can_be_weak: true,
                    boost_full_match: true,
                },
            ) {
                return score;
            }
        }
        Score {
            value: 0,
            word_start,
            positions: Vec::new(),
        }
    }

    fn score_text(
        &mut self,
        pattern: &Text,
        pattern_start: usize,
        word: &Text,
        word_start: usize,
        options: ScoreOptions,
    ) -> Option<Score> {
        let pattern_len = pattern.units.len().min(MAX_UNITS);
        let word_len = word.units.len().min(MAX_UNITS);
        if pattern_start >= pattern_len
            || word_start >= word_len
            || pattern_len - pattern_start > word_len - word_start
        {
            return None;
        }
        let mut pattern_position = pattern_start;
        for word_position in word_start..word_len {
            if pattern.lower.get(pattern_position) == word.lower.get(word_position) {
                self.minimum[pattern_position] = word_position;
                pattern_position += 1;
                if pattern_position == pattern_len {
                    break;
                }
            }
        }
        if pattern_position != pattern_len {
            return None;
        }
        let mut pattern_position = pattern_len - 1;
        for word_position in (word_start..word_len).rev() {
            if pattern.lower.get(pattern_position) == word.lower.get(word_position) {
                self.maximum[pattern_position] = word_position;
                if pattern_position == pattern_start {
                    break;
                }
                pattern_position -= 1;
            }
        }
        let mut strong_first = false;
        for pattern_position in pattern_start..pattern_len {
            let row = pattern_position - pattern_start + 1;
            let minimum = self.minimum[pattern_position];
            let maximum = self.maximum[pattern_position];
            let next_maximum = if pattern_position + 1 < pattern_len {
                self.maximum[pattern_position + 1]
            } else {
                word_len
            };
            for word_position in minimum..next_maximum {
                let column = word_position - word_start + 1;
                let cell = row * TABLE_SIDE + column;
                let diagonal_cell = (row - 1) * TABLE_SIDE + column - 1;
                let score = (word_position <= maximum)
                    .then(|| {
                        match_score(
                            pattern,
                            pattern_position,
                            pattern_start,
                            word,
                            word_position,
                            word_len,
                            word_start,
                            self.diagonals[diagonal_cell] == 0,
                            &mut strong_first,
                        )
                    })
                    .flatten();
                let diagonal_score = score.map(|score| score + self.scores[diagonal_cell]);
                let left_score = (word_position > minimum).then(|| {
                    self.scores[cell - 1]
                        - if self.diagonals[cell - 1] > 0 {
                            GAP_PENALTY
                        } else {
                            0
                        }
                });
                let left_left_score = (word_position > minimum + 1 && self.diagonals[cell - 1] > 0)
                    .then(|| {
                        self.scores[cell - LEFT_LEFT_DISTANCE]
                            - if self.diagonals[cell - LEFT_LEFT_DISTANCE] > 0 {
                                GAP_PENALTY
                            } else {
                                0
                            }
                    });
                if let Some(left_left) = left_left_score
                    && left_score.is_none_or(|left| left_left >= left)
                    && diagonal_score.is_none_or(|diagonal| left_left >= diagonal)
                {
                    self.scores[cell] = left_left;
                    self.arrows[cell] = LEFT_LEFT;
                    self.diagonals[cell] = 0;
                    continue;
                }
                if let Some(left) = left_score
                    && diagonal_score.is_none_or(|diagonal| left >= diagonal)
                {
                    self.scores[cell] = left;
                    self.arrows[cell] = LEFT;
                    self.diagonals[cell] = 0;
                    continue;
                }
                self.scores[cell] = diagonal_score?;
                self.arrows[cell] = DIAGONAL;
                self.diagonals[cell] = self.diagonals[diagonal_cell] + 1;
            }
        }
        if !strong_first && !options.first_match_can_be_weak {
            return None;
        }
        let mut row = pattern_len - pattern_start;
        let mut column = word_len - word_start;
        let mut result = Score {
            value: self.scores[row * TABLE_SIDE + column],
            word_start,
            positions: Vec::with_capacity(row),
        };
        let mut backwards = 0;
        let mut last_match = 0;
        while row > 0 {
            let mut diagonal = column;
            while diagonal > 0 {
                match self.arrows[row * TABLE_SIDE + diagonal] {
                    LEFT_LEFT => diagonal -= LEFT_LEFT_DISTANCE,
                    LEFT => diagonal -= 1,
                    _ => break,
                }
            }
            if backwards >= LONG_BACKWARDS_MATCH
                && pattern.lower.get(pattern_start + row - 1)
                    == word.lower.get(word_start + column - 1)
                && !word.is_uppercase(diagonal + word_start - 1)
                && backwards + 1 > self.diagonals[row * TABLE_SIDE + diagonal]
            {
                diagonal = column;
            }
            backwards = if diagonal == column { backwards + 1 } else { 1 };
            if last_match == 0 {
                last_match = diagonal;
            }
            row -= 1;
            column = diagonal.checked_sub(1)?;
            result.positions.push(column);
        }
        if word_len - word_start == pattern_len && options.boost_full_match {
            result.value += FULL_MATCH_BONUS;
        }
        result.value -= last_match as i32 - pattern_len as i32;
        Some(result)
    }
}

fn match_score(
    pattern: &Text,
    pattern_position: usize,
    pattern_start: usize,
    word: &Text,
    word_position: usize,
    word_len: usize,
    word_start: usize,
    new_match: bool,
    strong_first: &mut bool,
) -> Option<i32> {
    if pattern.lower.get(pattern_position) != word.lower.get(word_position) {
        return None;
    }
    let previous = word_position.checked_sub(1);
    let previous_separator = previous.is_some_and(|position| is_separator(&word.lower, position));
    let previous_whitespace =
        previous.is_some_and(|position| matches!(word.lower.get(position), Some(&SPACE | &TAB)));
    let mut score = 1;
    let mut preferred_gap = false;
    if word_position == pattern_position - pattern_start {
        score = if pattern.units.get(pattern_position) == word.units.get(word_position) {
            EXACT_BOUNDARY_SCORE
        } else {
            FOLDED_BOUNDARY_SCORE
        };
    } else if word.is_uppercase(word_position)
        && previous.is_none_or(|position| !word.is_uppercase(position))
    {
        score = if pattern.units.get(pattern_position) == word.units.get(word_position) {
            EXACT_BOUNDARY_SCORE
        } else {
            FOLDED_BOUNDARY_SCORE
        };
        preferred_gap = true;
    } else if is_separator(&word.lower, word_position) && !previous_separator {
        score = FOLDED_BOUNDARY_SCORE;
    } else if previous_separator || previous_whitespace {
        score = FOLDED_BOUNDARY_SCORE;
        preferred_gap = true;
    }
    if score > 1 && pattern_position == pattern_start {
        *strong_first = true;
    }
    preferred_gap |= word.is_uppercase(word_position) || previous_separator || previous_whitespace;
    if pattern_position == pattern_start {
        if word_position > word_start {
            score -= if preferred_gap {
                PREFERRED_GAP_PENALTY
            } else {
                GAP_PENALTY
            };
        }
    } else if new_match {
        if preferred_gap {
            score += NEW_BOUNDARY_BONUS;
        }
    } else if !preferred_gap {
        score += 1;
    }
    if word_position + 1 == word_len {
        score -= if preferred_gap {
            PREFERRED_GAP_PENALTY
        } else {
            GAP_PENALTY
        };
    }
    Some(score)
}

fn is_separator(units: &[u16], position: usize) -> bool {
    let Some(unit) = units.get(position).copied() else {
        return false;
    };
    let code = if (HIGH_SURROGATE_START..=HIGH_SURROGATE_END).contains(&unit)
        && let Some(low) = units
            .get(position + 1)
            .filter(|low| (LOW_SURROGATE_START..=LOW_SURROGATE_END).contains(low))
    {
        SURROGATE_BASE
            + ((u32::from(unit - HIGH_SURROGATE_START)) << SURROGATE_SHIFT)
            + u32::from(low - LOW_SURROGATE_START)
    } else {
        u32::from(unit)
    };
    matches!(code,
        95 | 45 | 46 | 32 | 47 | 92 | 39 | 34 | 58 | 36 | 60 | 62 | 40 | 41 | 91 | 93 | 123 | 125
        | 0x1f1e6..=0x1f1ff | 8986 | 8987 | 9200 | 9203 | 9728..=10175 | 11088 | 11093
        | 127744..=128591 | 128640..=128764 | 128992..=129008 | 129280..=129535 | 129648..=129782
    )
}
