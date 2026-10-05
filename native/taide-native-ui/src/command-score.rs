const CONTINUOUS: f64 = 1.0;
const SPACE_BOUNDARY: f64 = 0.9;
const PUNCTUATION_BOUNDARY: f64 = 0.8;
const GAP: f64 = 0.17;
const TRANSPOSE: f64 = 0.1;
const SKIPPED: f64 = 0.999;
const CASE: f64 = 0.9999;
const PREFIX: f64 = 0.99;
const TRANSPOSE_DISTANCE: usize = 2;

pub fn score(value: &str, search: &str) -> f64 {
    let original = value.encode_utf16().collect::<Vec<_>>();
    let query = search.encode_utf16().collect::<Vec<_>>();
    let folded = normalized(value);
    let folded_query = normalized(search);
    let width = folded.len() + 1;
    let mut scores = vec![0.0; (query.len() + 1) * width];
    for offset in 0..width {
        scores[query.len() * width + offset] = if offset == original.len() {
            CONTINUOUS
        } else {
            PREFIX
        };
    }
    for query_index in (0..query.len()).rev() {
        for offset in (0..width).rev() {
            let mut best: f64 = 0.0;
            for cursor in offset..folded.len() {
                if folded.get(cursor) != folded_query.get(query_index) {
                    continue;
                }
                let mut candidate = scores[(query_index + 1) * width + cursor + 1];
                if candidate > best {
                    if cursor != offset {
                        let previous = original.get(cursor - 1).copied();
                        if previous.is_some_and(is_punctuation) {
                            candidate *= PUNCTUATION_BOUNDARY;
                            if offset > 0 {
                                let count = original
                                    .get(offset..cursor - 1)
                                    .unwrap_or_default()
                                    .iter()
                                    .filter(|unit| is_punctuation(**unit))
                                    .count();
                                candidate *= SKIPPED.powf(count as f64);
                            }
                        } else if previous.is_some_and(is_space_or_dash) {
                            candidate *= SPACE_BOUNDARY;
                            if offset > 0 {
                                let count = original
                                    .get(offset..cursor - 1)
                                    .unwrap_or_default()
                                    .iter()
                                    .filter(|unit| is_space_or_dash(**unit))
                                    .count();
                                candidate *= SKIPPED.powf(count as f64);
                            }
                        } else {
                            candidate *= GAP;
                            if offset > 0 {
                                candidate *= SKIPPED.powf((cursor - offset) as f64);
                            }
                        }
                    }
                    if original.get(cursor) != query.get(query_index) {
                        candidate *= CASE;
                    }
                }
                let previous = cursor.checked_sub(1).and_then(|index| folded.get(index));
                let next_query = folded_query.get(query_index + 1);
                if ((candidate < TRANSPOSE && previous == next_query)
                    || (next_query == folded_query.get(query_index)
                        && previous != folded_query.get(query_index)))
                    && query_index + TRANSPOSE_DISTANCE <= query.len()
                {
                    candidate = candidate.max(
                        scores[(query_index + TRANSPOSE_DISTANCE) * width + cursor + 1] * TRANSPOSE,
                    );
                }
                best = best.max(candidate);
            }
            scores[query_index * width + offset] = best;
        }
    }
    scores[0]
}

fn normalized(value: &str) -> Vec<u16> {
    value
        .to_lowercase()
        .encode_utf16()
        .map(|unit| {
            if is_space_or_dash(unit) {
                u16::from(b' ')
            } else {
                unit
            }
        })
        .collect()
}

fn is_punctuation(unit: u16) -> bool {
    matches!(
        unit,
        92 | 47 | 95 | 43 | 46 | 35 | 34 | 64 | 91 | 40 | 123 | 38
    )
}

fn is_space_or_dash(unit: u16) -> bool {
    matches!(unit, 9..=13 | 32 | 45 | 160 | 5760 | 8192..=8202 | 8232 | 8233 | 8239 | 8287 | 12288 | 65279)
}
