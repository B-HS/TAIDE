use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};

use icu_collator::{Collator, CollatorBorrowed};
use icu_locale_core::Locale;
use icu_normalizer::ComposingNormalizer;
use taide_model::error::{AppError, AppResult};

use crate::keybinding_search::js_whitespace;

const MAX_QUERY_TOKENS: usize = 8;
const MATCH_BASE_SCORE: usize = 1;
const CONSECUTIVE_MATCH_BONUS: usize = 5;
const PATH_SEGMENT_SEPARATOR: char = '/';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub score: usize,
    pub indices: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ranked<T> {
    pub item: T,
    pub label: String,
    pub matched: Match,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment<'a> {
    pub text: &'a str,
    pub is_matched: bool,
}

pub struct Matcher {
    collator: CollatorBorrowed<'static>,
}

impl Matcher {
    pub fn new(locale: &str) -> AppResult<Self> {
        let locale = locale
            .parse::<Locale>()
            .map_err(|error| AppError::Internal(format!("native collation locale: {error}")))?;
        let collator = Collator::try_new(locale.into(), Default::default())
            .map_err(|error| AppError::Internal(format!("native collation data: {error}")))?;
        Ok(Self { collator })
    }

    pub fn filter<T>(
        &self,
        query: &str,
        items: impl IntoIterator<Item = T>,
        label_of: impl for<'a> Fn(&'a T) -> Cow<'a, str>,
        limit: Option<usize>,
    ) -> Vec<Ranked<T>> {
        let query = normalize(query);
        let tokens = query
            .split(js_whitespace)
            .filter(|token| !token.is_empty())
            .take(MAX_QUERY_TOKENS)
            .collect::<Vec<_>>();
        let limit = limit.unwrap_or(usize::MAX);
        let ranked = items.into_iter().filter_map(|item| {
            let (label, matched) = {
                let source = label_of(&item);
                let normalized = normalize(&source);
                let matched = match_tokens(&tokens, &normalized)?;
                (normalized.into_owned(), matched)
            };
            Some(Ranked {
                item,
                label,
                matched,
            })
        });
        if tokens.is_empty() {
            return ranked.take(limit).collect();
        }
        let mut ranked = ranked.collect::<Vec<_>>();
        ranked.sort_by(|left, right| self.compare(left, right));
        ranked.truncate(limit);
        ranked
    }

    fn compare<T>(&self, left: &Ranked<T>, right: &Ranked<T>) -> Ordering {
        right
            .matched
            .score
            .cmp(&left.matched.score)
            .then_with(|| reaches_last_segment(right).cmp(&reaches_last_segment(left)))
            .then_with(|| utf16_len(&left.label).cmp(&utf16_len(&right.label)))
            .then_with(|| self.collator.compare(&left.label, &right.label))
    }
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
        if character.to_lowercase().eq(expected.chars()) {
            score += MATCH_BASE_SCORE;
            if previous.is_some_and(|previous| position == previous + 1) {
                score += CONSECUTIVE_MATCH_BONUS;
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

pub fn highlight_segments<'a>(text: &'a str, indices: &[usize]) -> Vec<Segment<'a>> {
    let matched = indices.iter().copied().collect::<HashSet<_>>();
    let mut segments = Vec::new();
    let mut start = 0;
    let mut unit_index = 0;
    let mut current = None;
    for (offset, character) in text.char_indices() {
        let is_matched = matched.contains(&unit_index);
        unit_index += character.len_utf16();
        match current {
            Some(previous) if previous != is_matched => {
                segments.push(Segment {
                    text: &text[start..offset],
                    is_matched: previous,
                });
                start = offset;
            }
            _ => (),
        }
        current = Some(is_matched);
    }
    if let Some(is_matched) = current {
        segments.push(Segment {
            text: &text[start..],
            is_matched,
        });
    }
    segments
}

pub(crate) fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

fn normalize(value: &str) -> Cow<'_, str> {
    if value.is_ascii() {
        return Cow::Borrowed(value);
    }
    ComposingNormalizer::new_nfc().normalize(value)
}

fn match_tokens(tokens: &[&str], target: &str) -> Option<Match> {
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

fn reaches_last_segment<T>(ranked: &Ranked<T>) -> bool {
    let Some(last) = ranked.matched.indices.last() else {
        return false;
    };
    match ranked.label.rfind(PATH_SEGMENT_SEPARATOR) {
        Some(separator) => *last > utf16_len(&ranked.label[..separator]),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icu_normalizer::DecomposingNormalizer;

    const COLLATION_LOCALE: &str = "en-US";
    const ROWS: [&str; 3] = [
        "src/widgets/editor-area/pane-node-view.tsx",
        "src/entities/plugin/plugin.query.ts",
        "src/shared/lib/fuzzy-match.ts",
    ];

    fn indices(query: &str, target: &str) -> Option<Vec<usize>> {
        fuzzy_match(query, target).map(|matched| matched.indices)
    }

    fn score(query: &str, target: &str) -> usize {
        fuzzy_match(query, target).unwrap().score
    }

    fn code_units(target: &str, indices: &[usize]) -> String {
        let units = target.encode_utf16().collect::<Vec<_>>();
        String::from_utf16(
            &indices
                .iter()
                .map(|index| units[*index])
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }

    fn decomposed(value: &str) -> String {
        DecomposingNormalizer::new_nfd()
            .normalize(value)
            .into_owned()
    }

    fn filter<'a>(query: &str, paths: &[&'a str]) -> Vec<Ranked<&'a str>> {
        Matcher::new(COLLATION_LOCALE).unwrap().filter(
            query,
            paths.iter().copied(),
            |path| Cow::Borrowed(*path),
            None,
        )
    }

    fn rank<'a>(query: &str, paths: &[&'a str]) -> Vec<&'a str> {
        filter(query, paths)
            .into_iter()
            .map(|ranked| ranked.item)
            .collect()
    }

    fn segments<'a>(text: &'a str, indices: &[usize]) -> Vec<(&'a str, bool)> {
        highlight_segments(text, indices)
            .into_iter()
            .map(|segment| (segment.text, segment.is_matched))
            .collect()
    }

    #[test]
    fn fuzzy_match는_순서대로_나오는_부분_문자를_대소문자_구분없이_매칭한다() {
        assert_eq!(
            fuzzy_match("", "anything.ts"),
            Some(Match {
                score: 0,
                indices: Vec::new()
            }),
            "빈 쿼리는 항상 매칭되고 점수 0 을 반환한다"
        );
        assert_eq!(
            indices("pnv", "pane-node-view.tsx"),
            Some(vec![0, 2, 10]),
            "부분 문자가 순서대로 존재하면 매칭된다"
        );
        assert_eq!(
            indices("vnp", "pane-node-view.tsx"),
            None,
            "순서가 어긋나면 매칭되지 않는다"
        );
        assert_eq!(
            indices("xyz", "pane-node-view.tsx"),
            None,
            "타겟에 없는 문자가 있으면 매칭되지 않는다"
        );
        assert!(fuzzy_match("PNV", "pane-node-view.tsx").is_some());
        assert!(fuzzy_match("pnv", "Pane-Node-View.tsx").is_some());
        assert_eq!(indices("a", ""), None, "타겟이 비어 있으면 매칭되지 않는다");
        assert_eq!(
            fuzzy_match("", ""),
            Some(Match {
                score: 0,
                indices: Vec::new()
            })
        );
        assert_eq!(
            indices("abcd", "abc"),
            None,
            "쿼리가 타겟보다 길면 매칭되지 않는다"
        );
        assert_eq!(
            indices("aa", "abaca"),
            Some(vec![0, 2]),
            "같은 문자가 반복되면 항상 가장 앞의 남은 위치를 집는다"
        );
        let target = "src/widgets/editor-area/pane-node-view.tsx";
        let matched = indices("pnv", target).unwrap();
        assert!(matched.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            code_units(target, &matched),
            "pnv",
            "매칭 인덱스는 항상 증가하고 타겟 코드유닛 오프셋으로 그대로 슬라이스된다"
        );
    }

    #[test]
    fn fuzzy_match는_연속_매칭에_더_높은_점수를_준다() {
        assert!(
            score("pan", "pane.ts") > score("pts", "pane.ts"),
            "연속 매칭은 비연속 매칭보다 점수가 높다"
        );
        assert!(
            score("editor", "editor-pane.tsx") > score("edtr", "editor-pane.tsx"),
            "완전한 연속 일치가 부분 연속 일치보다 점수가 높다"
        );
        assert!(
            score("a🚀", "a🚀b") > score("a🚀", "axx🚀"),
            "서로게이트 쌍도 코드포인트 1칸으로 세어 연속 매칭 보너스를 받는다"
        );
    }

    #[test]
    fn fuzzy_match_인덱스는_utf16_코드유닛_오프셋이다() {
        assert_eq!(
            indices("🚀", "src/🚀rocket.ts"),
            Some(vec![4, 5]),
            "서로게이트 쌍(이모지)에 매칭되면 두 코드유닛 인덱스가 모두 담긴다"
        );
        assert_eq!(
            indices("s🚀", "ship🚀"),
            Some(vec![0, 4, 5]),
            "서로게이트 쌍이 타겟 끝에 있어도 두 코드유닛이 모두 담긴다"
        );
        assert_eq!(code_units("ship🚀", &[4, 5]), "🚀");
        assert_eq!(
            indices("🚀🎉", "🚀a🎉"),
            Some(vec![0, 1, 3, 4]),
            "서로게이트 쌍이 여러 개면 각각의 코드유닛 오프셋을 정확히 가리킨다"
        );
        assert_eq!(
            indices("b", "🚀ab"),
            Some(vec![3]),
            "서로게이트 쌍 뒤의 문자 인덱스가 코드유닛 기준으로 밀린다"
        );
        assert_eq!(code_units("🚀ab", &[3]), "b");
        assert_eq!(
            segments(
                "src/🚀rocket.ts",
                &indices("🚀", "src/🚀rocket.ts").unwrap()
            ),
            [("src/", false), ("🚀", true), ("rocket.ts", false)],
            "서로게이트 쌍 매칭 인덱스로 세그먼트를 나누면 문자가 쪼개지지 않는다"
        );
    }

    #[test]
    fn fuzzy_match는_소문자화로_길이가_늘어나는_문자_뒤의_인덱스를_밀지_않는다() {
        assert_eq!(indices("t", "İstanbul.ts"), Some(vec![2]));
        assert_eq!(code_units("İstanbul.ts", &[2]), "t");
        assert_eq!(
            indices("i", "İstanbul"),
            None,
            "İ 는 소문자형이 2코드유닛이라 단일 문자 'i' 쿼리와 매칭되지 않는다"
        );
        assert_eq!(indices("i", "İstanbul.min.ts"), Some(vec![10]));
        assert_eq!(code_units("İstanbul.min.ts", &[10]), "i");
        assert_eq!(
            indices("İ", "İstanbul.ts"),
            Some(vec![0]),
            "İ 자신을 쿼리로 주면 코드포인트끼리 비교되어 매칭된다"
        );
        assert_eq!(
            indices("sd", "src/İd.ts"),
            Some(vec![0, 5]),
            "İ 가 중간에 있어도 뒤 문자들의 코드유닛 오프셋이 그대로 유지된다"
        );
        assert_eq!(code_units("src/İd.ts", &[5]), "d");
    }

    #[test]
    fn fuzzy_filter는_토큰별로_매칭하고_점수순으로_정렬한다() {
        assert!(
            filter("zzzz", &ROWS).is_empty(),
            "매칭되지 않는 항목은 제외한다"
        );
        assert_eq!(
            rank("fuzzy", &ROWS).first(),
            Some(&ROWS[2]),
            "점수 높은 순으로 정렬한다"
        );
        assert_eq!(
            rank("", &ROWS),
            ROWS,
            "빈 쿼리는 전체 항목을 원래 순서로 반환한다"
        );
        assert_eq!(
            rank("widgets pane", &ROWS),
            [ROWS[0]],
            "공백으로 나뉜 토큰이 각각 매칭되면 통과한다"
        );
        assert_eq!(
            filter("pane widgets", &ROWS),
            filter("widgets pane", &ROWS),
            "토큰 순서가 라벨 등장 순서와 달라도 매칭된다"
        );
        assert_eq!(rank("view editor", &ROWS), [ROWS[0]]);
        assert!(
            filter("widgets zzzz", &ROWS).is_empty(),
            "토큰이 하나라도 매칭되지 않으면 탈락한다"
        );
        assert_eq!(
            filter("widgets pane", &ROWS)[0].matched.score,
            score("widgets", ROWS[0]) + score("pane", ROWS[0]),
            "여러 토큰의 점수는 각 토큰 점수의 합이다"
        );
        assert_eq!(
            filter("widgets pane", &ROWS)[0].matched.indices,
            [4, 5, 6, 7, 8, 9, 10, 24, 25, 26, 27],
            "여러 토큰의 인덱스는 정렬된 합집합이다"
        );
        assert_eq!(
            filter("fuzzy fu", &ROWS)[0].matched.indices,
            indices("fuzzy", ROWS[2]).unwrap(),
            "겹치는 토큰도 중복되지 않는다"
        );
        assert_eq!(
            filter("  widgets   pane  ", &ROWS),
            filter("widgets pane", &ROWS),
            "앞뒤·연속 공백은 무시한다"
        );
        assert_eq!(filter(" fuzzy ", &ROWS), filter("fuzzy", &ROWS));
        assert_eq!(rank("   ", &ROWS), ROWS);
        assert!(
            filter("s r c h l i b zzzz", &ROWS).is_empty(),
            "토큰 상한(8개) 안의 토큰은 모두 매칭되어야 한다"
        );
        assert_eq!(
            rank("s r c h l i b f zzzz", &ROWS),
            [ROWS[2]],
            "토큰 상한(8개)을 넘는 토큰은 무시한다"
        );
        let single = filter("fuzzy", &ROWS);
        assert_eq!(rank("fuzzy", &ROWS), [ROWS[2]]);
        assert_eq!(
            Some(&single[0].matched),
            fuzzy_match("fuzzy", ROWS[2]).as_ref(),
            "단일 토큰 결과는 fuzzy_match 결과 그대로다"
        );
    }

    #[test]
    fn fuzzy_filter_동점은_파일명_매치_짧은_라벨_사전순으로_고정된다() {
        let directory = "src/ab/x.ts";
        let file_name = "src/x/ab.ts";
        assert_eq!(
            rank("ab", &[directory, file_name]),
            [file_name, directory],
            "점수가 같으면 파일명 매치가 디렉토리 매치보다 앞선다"
        );
        assert_eq!(rank("ab", &[file_name, directory]), [file_name, directory]);
        assert_eq!(
            rank("ab", &["ab.tsx", "ab.ts"]),
            ["ab.ts", "ab.tsx"],
            "점수·매치 위치가 같으면 짧은 라벨이 앞선다"
        );
        assert_eq!(rank("ab", &["ab.ts", "ab.tsx"]), ["ab.ts", "ab.tsx"]);
        assert_eq!(
            rank("ab", &["ab.ts", "ab.js"]),
            ["ab.js", "ab.ts"],
            "길이까지 같으면 라벨 사전순으로 고정된다"
        );
        assert_eq!(rank("ab", &["ab.js", "ab.ts"]), ["ab.js", "ab.ts"]);
        let paths = ["src/ab/x.ts", "src/x/ab.ts", "ab.ts", "ab.js", "a/b.ts"];
        let reversed = paths.iter().rev().copied().collect::<Vec<_>>();
        assert_eq!(
            rank("ab", &paths),
            rank("ab", &reversed),
            "같은 후보 집합은 입력 순서가 달라도 같은 순위를 낸다"
        );
        let unsorted = ["zzz.ts", "a.ts", "src/m.ts"];
        assert_eq!(
            rank("", &unsorted),
            unsorted,
            "빈 질의는 정렬하지 않고 입력 순서를 그대로 돌려준다"
        );
        assert_eq!(rank("   ", &unsorted), unsorted);
    }

    #[test]
    fn fuzzy_filter는_질의와_라벨을_nfc로_맞추고_item은_원본으로_둔다() {
        let composed = "src/한글/파일.ts";
        let decomposed_path = decomposed(composed);
        assert_ne!(decomposed_path, composed);
        assert_eq!(
            filter("파일", &[decomposed_path.as_str()]).len(),
            1,
            "NFD 라벨도 NFC 질의로 매칭된다"
        );
        assert_eq!(
            filter(&decomposed("파일"), &[composed]).len(),
            1,
            "NFD 질의도 NFC 라벨을 찾는다"
        );
        let ranked = filter("파일", &[decomposed_path.as_str()]).remove(0);
        assert_eq!(ranked.item, decomposed_path, "item 은 원본 그대로 둔다");
        assert_eq!(ranked.label, composed, "label 만 NFC 정규화본이다");
        assert_eq!(
            segments(&ranked.label, &ranked.matched.indices),
            [("src/한글/", false), ("파일", true), (".ts", false)],
            "매칭 인덱스는 label(정규화본) 기준이라 강조가 질의와 정확히 겹친다"
        );
        assert_eq!(
            filter("pnv", &["pane-node-view.tsx"])[0].label,
            "pane-node-view.tsx",
            "ASCII 라벨은 정규화를 거쳐도 동일 문자열이다"
        );
    }

    #[test]
    fn fuzzy_filter_상한은_정렬된_결과의_앞부분만_남긴다() {
        let matcher = Matcher::new(COLLATION_LOCALE).unwrap();
        let limited = |query: &str, limit| {
            matcher
                .filter(
                    query,
                    ROWS.iter().copied(),
                    |path| Cow::Borrowed(*path),
                    Some(limit),
                )
                .into_iter()
                .map(|ranked| ranked.item)
                .collect::<Vec<_>>()
        };
        assert_eq!(limited("", 2), ROWS[..2]);
        assert_eq!(limited("s", 1), rank("s", &ROWS)[..1]);
        assert_eq!(limited("s", ROWS.len() + 1), rank("s", &ROWS));
        assert!(Matcher::new("not/a/locale").is_err());
    }

    #[test]
    fn highlight_segments는_연속된_매칭과_비매칭을_묶는다() {
        assert_eq!(
            segments("index.ts", &[]),
            [("index.ts", false)],
            "매칭 인덱스가 없으면 전체를 비매칭 세그먼트 하나로 반환한다"
        );
        assert!(
            segments("", &[0, 1]).is_empty(),
            "빈 문자열은 빈 배열을 반환한다"
        );
        assert_eq!(
            segments("index.ts", &[0, 1, 2]),
            [("ind", true), ("ex.ts", false)],
            "연속된 매칭 인덱스를 하나의 세그먼트로 묶는다"
        );
        assert_eq!(
            segments(
                "pane-node-view.tsx",
                &indices("pnv", "pane-node-view.tsx").unwrap()
            ),
            [
                ("p", true),
                ("a", false),
                ("n", true),
                ("e-node-", false),
                ("v", true),
                ("iew.tsx", false)
            ],
            "흩어진 매칭 인덱스마다 별도 세그먼트를 만든다"
        );
        assert_eq!(
            segments("abc", &[0, 1, 2]),
            [("abc", true)],
            "전체 문자가 매칭되면 세그먼트 하나로 반환한다"
        );
        assert_eq!(
            segments("abc", &[2]),
            [("ab", false), ("c", true)],
            "마지막 문자만 매칭되면 마지막 세그먼트만 매칭 표시한다"
        );
    }
}
