pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 같은_바이트열은_상수시간_비교에서_참이다() {
        assert!(constant_time_eq(b"abc123", b"abc123"));
    }

    #[test]
    fn 길이가_다르면_거짓이다() {
        assert!(!constant_time_eq(b"short", b"longer-value"));
    }

    #[test]
    fn 내용이_다르면_거짓이다() {
        assert!(!constant_time_eq(b"abc123", b"abc124"));
    }
}
