use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeSet;

use icu_collator::{Collator, CollatorBorrowed};
use icu_locale_core::Locale;
use icu_normalizer::ComposingNormalizer;
use taide_model::error::{AppError, AppResult};
use taide_model::locale::ResolvedLocale;

use crate::command_registry::format_categorized_label;
use crate::keymap::catalog::Row;

const MAX_QUERY_TOKENS: usize = 8;
const MATCH_SCORE: usize = 1;
const CONSECUTIVE_BONUS: usize = 5;

pub struct Search {
    collator: CollatorBorrowed<'static>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    pub score: usize,
    pub indices: Vec<usize>,
}

pub struct Ranked {
    pub index: usize,
    pub label: String,
    pub matched: Match,
}

impl Search {
    pub fn new(locale: &str) -> AppResult<Self> {
        let locale = locale
            .parse::<Locale>()
            .map_err(|error| AppError::Internal(format!("native collation locale: {error}")))?;
        let collator = Collator::try_new(locale.into(), Default::default())
            .map_err(|error| AppError::Internal(format!("native collation data: {error}")))?;
        Ok(Self { collator })
    }

    pub fn compare(&self, left: &str, right: &str) -> Ordering {
        self.collator.compare(left, right)
    }

    pub fn ordered_indices(&self, rows: &[Row], locale: &ResolvedLocale) -> Vec<usize> {
        let labels = rows
            .iter()
            .map(|row| label(row, locale))
            .collect::<Vec<_>>();
        let mut order = (0..rows.len()).collect::<Vec<_>>();
        order.sort_by(|left, right| {
            rows[*left]
                .binding
                .key()
                .is_empty()
                .cmp(&rows[*right].binding.key().is_empty())
                .then_with(|| self.compare(&labels[*left], &labels[*right]))
        });
        order
    }

    pub fn filter(&self, query: &str, labels: &[String]) -> Vec<Ranked> {
        let query = normalize(query);
        let tokens = query
            .split(js_whitespace)
            .filter(|token| !token.is_empty())
            .take(MAX_QUERY_TOKENS)
            .collect::<Vec<_>>();
        let mut result = labels
            .iter()
            .enumerate()
            .filter_map(|(index, original)| {
                let label = normalize(original).into_owned();
                let matched = tokens_match(&tokens, &label)?;
                Some(Ranked {
                    index,
                    label,
                    matched,
                })
            })
            .collect::<Vec<_>>();
        if tokens.is_empty() {
            return result;
        }
        result.sort_by(|left, right| {
            right
                .matched
                .score
                .cmp(&left.matched.score)
                .then_with(|| last_segment(right).cmp(&last_segment(left)))
                .then_with(|| {
                    left.label
                        .encode_utf16()
                        .count()
                        .cmp(&right.label.encode_utf16().count())
                })
                .then_with(|| self.compare(&left.label, &right.label))
        });
        result
    }
}

pub fn label(row: &Row, locale: &ResolvedLocale) -> String {
    format_categorized_label(
        locale,
        row.category_key.as_deref(),
        &row.title_key,
        row.title_default_value.as_deref(),
    )
}

fn normalize(value: &str) -> Cow<'_, str> {
    if value.is_ascii() {
        return Cow::Borrowed(value);
    }
    ComposingNormalizer::new_nfc().normalize(value)
}

pub(crate) fn js_whitespace(value: char) -> bool {
    matches!(value, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub fn fuzzy_match(query: &str, target: &str) -> Option<Match> {
    let mut query = query.chars();
    let Some(first) = query.next() else {
        return Some(Match {
            score: 0,
            indices: Vec::new(),
        });
    };
    let mut expected = first.to_lowercase().collect::<String>();
    let mut score = 0;
    let mut indices = Vec::new();
    let mut previous = None;
    let mut unit_index = 0;
    for (position, character) in target.chars().enumerate() {
        if character.to_lowercase().collect::<String>() == expected {
            score += MATCH_SCORE;
            if previous.is_some_and(|previous| position == previous + 1) {
                score += CONSECUTIVE_BONUS;
            }
            indices.extend(unit_index..unit_index + character.len_utf16());
            previous = Some(position);
            let Some(next) = query.next() else {
                return Some(Match { score, indices });
            };
            expected = next.to_lowercase().collect();
        }
        unit_index += character.len_utf16();
    }
    None
}

fn tokens_match(tokens: &[&str], target: &str) -> Option<Match> {
    if tokens.len() <= 1 {
        return fuzzy_match(tokens.first().copied().unwrap_or_default(), target);
    }
    let mut indices = BTreeSet::new();
    let mut score = 0;
    for token in tokens {
        let matched = fuzzy_match(token, target)?;
        score += matched.score;
        indices.extend(matched.indices);
    }
    Some(Match {
        score,
        indices: indices.into_iter().collect(),
    })
}

fn last_segment(value: &Ranked) -> bool {
    let Some(last) = value.matched.indices.last() else {
        return false;
    };
    match value.label.rfind('/') {
        Some(slash) => *last > value.label[..slash].encode_utf16().count(),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::catalog::{Overrides, rows};
    use serde_json::{Value, json};

    const FIXTURE: &str =
        include_str!("../../taide-native-app/tests/fixtures/keybinding-search.json");

    fn matched_json(matched: &Match) -> Value {
        json!({"score":matched.score,"indices":matched.indices})
    }

    #[test]
    fn keybinding_search는_원본_nfc_utf16_토큰과_전체3언어정렬을_보존한다() {
        let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        let search = Search::new(fixture["locale"].as_str().unwrap()).unwrap();
        for raw in fixture["raw"].as_array().unwrap() {
            let result = fuzzy_match(
                raw["query"].as_str().unwrap(),
                raw["target"].as_str().unwrap(),
            );
            assert_eq!(
                result.as_ref().map(matched_json).unwrap_or(Value::Null),
                raw["matched"],
                "raw: {raw}"
            );
        }
        let labels = fixture["labels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        for case in fixture["fuzzy"].as_array().unwrap() {
            let result = search.filter(case["query"].as_str().unwrap(), &labels).iter().map(|ranked| json!({"item":labels[ranked.index],"label":ranked.label,"match":matched_json(&ranked.matched)})).collect::<Vec<_>>();
            assert_eq!(
                json!(result),
                case["ranked"],
                "fuzzy query {}",
                case["query"]
            );
        }
        let rows = rows(&Overrides::default(), true).unwrap();
        for catalog in fixture["catalogs"].as_array().unwrap() {
            let language = catalog["language"].as_str().unwrap();
            let pack = match language {
                "en" => include_str!("../../../crates/taide-locale/resources/locales/en.json"),
                "ko" => include_str!("../../../crates/taide-locale/resources/locales/ko.json"),
                "ja" => include_str!("../../../crates/taide-locale/resources/locales/ja.json"),
                _ => panic!("unknown fixture language"),
            };
            let locale = ResolvedLocale {
                id: language.into(),
                name: language.into(),
                messages: serde_json::from_str(pack).unwrap(),
                warnings: Vec::new(),
            };
            let labels = rows
                .iter()
                .map(|row| json!({"id":row.id,"key":row.binding.key(),"label":label(row,&locale)}))
                .collect::<Vec<_>>();
            assert_eq!(
                json!(labels),
                catalog["labels"],
                "{language}: localized labels"
            );
            let order = search.ordered_indices(&rows, &locale);
            assert_eq!(
                json!(
                    order
                        .iter()
                        .map(|index| rows[*index].id.as_str())
                        .collect::<Vec<_>>()
                ),
                catalog["sorted"],
                "{language}: complete sort"
            );
            let searchable = order
                .iter()
                .map(|index| format!("{} {}", label(&rows[*index], &locale), rows[*index].id))
                .collect::<Vec<_>>();
            for query in catalog["queries"].as_array().unwrap() {
                let result = search.filter(query["query"].as_str().unwrap(), &searchable);
                assert_eq!(
                    json!(
                        result
                            .iter()
                            .map(|ranked| rows[order[ranked.index]].id.as_str())
                            .collect::<Vec<_>>()
                    ),
                    query["ids"],
                    "{language}: query {}",
                    query["query"]
                );
            }
        }
        assert!(Search::new("not/a/locale").is_err());
    }
}
