use taide_native_editor::completion_filter::{Score, ScoreOptions, Scorer};

const HEX_RADIX: u32 = 16;
const HEX_PAIR_SIZE: usize = 2;
const FIELD_COUNT: usize = 10;

fn text(hex: &str) -> String {
    let bytes = hex
        .as_bytes()
        .chunks_exact(HEX_PAIR_SIZE)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), HEX_RADIX).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn 후보필터의_경계_camel_오타_unicode와_강조는_실제_monaco표본과_일치한다() {
    let mut scorer = Scorer::default();
    let mut count = 0;
    for row in include_str!("fixtures/completion-filter-reference.tsv").lines() {
        let fields = row.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), FIELD_COUNT);
        let pattern = text(fields[1]);
        let word = text(fields[2]);
        let pattern_start = fields[3].parse().unwrap();
        let word_start = fields[4].parse().unwrap();
        let options = ScoreOptions {
            first_match_can_be_weak: fields[5] == "1",
            boost_full_match: fields[6] == "1",
        };
        let actual = match fields[0] {
            "normal" => scorer.score(&pattern, pattern_start, &word, word_start, options),
            "graceful" => scorer.graceful(&pattern, pattern_start, &word, word_start, options),
            "any" => Some(scorer.any(&pattern, pattern_start, &word, word_start)),
            method => panic!("unknown method {method}"),
        };
        let expected = (fields[7] != "none").then(|| Score {
            value: fields[7].parse().unwrap(),
            word_start: fields[8].parse().unwrap(),
            positions: fields[9]
                .split(',')
                .filter(|value| *value != "none")
                .map(|value| value.parse().unwrap())
                .collect(),
        });
        assert_eq!(
            actual, expected,
            "sample {count}: {pattern:?}/{word:?}, {row}"
        );
        if let Some(score) = actual {
            let flattened = score.highlights().into_iter().flatten().collect::<Vec<_>>();
            let expected_highlights = score
                .positions
                .iter()
                .rev()
                .map(|position| position + word_start)
                .collect::<Vec<_>>();
            assert_eq!(flattened, expected_highlights);
        }
        count += 1;
    }
    assert!(count > 0);
}
