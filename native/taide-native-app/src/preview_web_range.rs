const MAX_HEADER_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Full,
    Partial { start: u64, length: u64 },
    Unsatisfiable,
}

fn decimal(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(value.bytes().fold(0u64, |number, byte| {
        number
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'))
    }))
}

pub fn select(header: Option<&str>, size: u64) -> Selection {
    let Some(header) = header.filter(|header| header.len() <= MAX_HEADER_BYTES) else {
        return Selection::Full;
    };
    let Some((unit, value)) = header.trim().split_once('=') else {
        return Selection::Full;
    };
    if !unit.eq_ignore_ascii_case("bytes") || value.contains(',') || size == 0 {
        return Selection::Full;
    }
    let Some((first, last)) = value.trim().split_once('-') else {
        return Selection::Full;
    };
    if first.is_empty() {
        return match decimal(last) {
            Some(0) => Selection::Unsatisfiable,
            Some(length) => {
                let length = length.min(size);
                Selection::Partial {
                    start: size - length,
                    length,
                }
            }
            None => Selection::Full,
        };
    }
    let Some(start) = decimal(first) else {
        return Selection::Full;
    };
    let end = match last {
        "" => size - 1,
        value => match decimal(value) {
            Some(end) if end >= start => end.min(size - 1),
            _ => return Selection::Full,
        },
    };
    if start >= size {
        return Selection::Unsatisfiable;
    }
    Selection::Partial {
        start,
        length: end - start + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_range는_closed_open_suffix_unsatisfiable와_overflow를_구분한다() {
        for (header, expected) in [
            (
                "bytes=0-0",
                Selection::Partial {
                    start: 0,
                    length: 1,
                },
            ),
            (
                "bytes=2-4",
                Selection::Partial {
                    start: 2,
                    length: 3,
                },
            ),
            (
                "bytes=7-",
                Selection::Partial {
                    start: 7,
                    length: 3,
                },
            ),
            (
                "bytes=-3",
                Selection::Partial {
                    start: 7,
                    length: 3,
                },
            ),
            (
                "bytes=-99",
                Selection::Partial {
                    start: 0,
                    length: 10,
                },
            ),
            (
                "bytes=7-99",
                Selection::Partial {
                    start: 7,
                    length: 3,
                },
            ),
            (
                "bytes=7-999999999999999999999999999",
                Selection::Partial {
                    start: 7,
                    length: 3,
                },
            ),
            (
                "bytes=-999999999999999999999999999",
                Selection::Partial {
                    start: 0,
                    length: 10,
                },
            ),
            (
                "bytes=999999999999999999999999999-",
                Selection::Unsatisfiable,
            ),
            ("bytes=10-", Selection::Unsatisfiable),
            ("bytes=-0", Selection::Unsatisfiable),
            ("bytes=8-7", Selection::Full),
            ("bytes=0-0,9-9", Selection::Full),
            ("items=0-2", Selection::Full),
            ("bytes=+1-", Selection::Full),
            ("bytes=1 - 2", Selection::Full),
        ] {
            assert_eq!(select(Some(header), 10), expected, "{header}");
        }
        assert_eq!(select(None, 10), Selection::Full);
        assert_eq!(select(Some("bytes=-1"), 0), Selection::Full);
        assert_eq!(
            select(Some(&"0".repeat(MAX_HEADER_BYTES + 1)), 10),
            Selection::Full
        );
        assert_eq!(
            select(Some("bytes=0-"), u64::MAX),
            Selection::Partial {
                start: 0,
                length: u64::MAX
            }
        );
    }
}
