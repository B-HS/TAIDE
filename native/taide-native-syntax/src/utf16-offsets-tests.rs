use super::{Utf16ByteCursor, utf16_len};

const MIXED_TEXT: &str = "a한😀e\u{301}z";
const MIXED_UTF16_STARTS: [usize; 6] = [0, 1, 2, 4, 5, 6];
const MIXED_BYTE_STARTS: [usize; 6] = [0, 1, 4, 8, 9, 11];
const MIXED_UTF16_LENGTH: usize = 7;
const PAST_END_UTF16_OFFSET: usize = 100;

#[test]
fn utf16_길이는_보조_평면_문자를_두_단위로_센다() {
    assert_eq!(utf16_len(MIXED_TEXT), MIXED_UTF16_LENGTH);
    assert_eq!(utf16_len(""), 0);
}

#[test]
fn 오름차순_utf16_위치를_문자_경계의_utf8_바이트_위치로_바꾼다() {
    let mut cursor = Utf16ByteCursor::new(MIXED_TEXT);
    let bytes: Vec<usize> = MIXED_UTF16_STARTS
        .iter()
        .map(|offset| cursor.byte_offset(*offset))
        .collect();
    assert_eq!(bytes, MIXED_BYTE_STARTS);
    assert!(
        bytes
            .iter()
            .all(|offset| MIXED_TEXT.is_char_boundary(*offset))
    );
}

#[test]
fn 줄_끝을_넘는_위치는_줄_길이로_멈춘다() {
    let mut cursor = Utf16ByteCursor::new(MIXED_TEXT);
    assert_eq!(cursor.byte_offset(MIXED_UTF16_LENGTH), MIXED_TEXT.len());
    assert_eq!(cursor.byte_offset(PAST_END_UTF16_OFFSET), MIXED_TEXT.len());
}
