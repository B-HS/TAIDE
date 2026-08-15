/// Escapes `value` for safe embedding as a single literal word in a POSIX-family
/// shell command line, including under shells (fish) that reinterpret escapes
/// even inside single quotes — see `docs/research/xterm-pty.md` §8 follow-up
/// notes for the reproduction that motivated the `\` handling below.
///
/// Wraps `value` in single quotes and closes/escapes/reopens the quoting
/// around any embedded `'` (the standard `'\''` POSIX idiom). Every raw `\`
/// is additionally doubled to `\\`. In POSIX sh/bash/zsh this doubling is a
/// no-op on the resulting argument value, because single quotes make *every*
/// byte — including `\` — completely literal there. fish does not follow
/// that rule: per its own docs, `\'` and `\\` are live escapes even inside
/// single quotes. Without doubling, a raw `\` that happens to immediately
/// precede an embedded `'` in the input would combine with this function's
/// own `'\''` idiom to form `\'` under fish's reading, closing the quote
/// early and letting anything after it (e.g. `; rm -rf ~`) run as a separate
/// command — doubling neutralizes that while staying inert for POSIX shells.
pub fn posix_quote(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('\'');
    for ch in value.chars() {
        match ch {
            '\'' => quoted.push_str("'\\''"),
            '\\' => quoted.push_str("\\\\"),
            _ => quoted.push(ch),
        }
    }
    quoted.push('\'');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 평범한_단어는_홑따옴표로만_감싼다() {
        assert_eq!(posix_quote("test"), "'test'");
    }

    #[test]
    fn 콜론이_포함된_이름도_그대로_보존한다() {
        assert_eq!(posix_quote("test:unit"), "'test:unit'");
    }

    #[test]
    fn 내장된_홑따옴표는_닫고_이스케이프하고_다시_연다() {
        assert_eq!(posix_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn 세미콜론과_명령_연결자는_리터럴로_보존된다() {
        assert_eq!(posix_quote("build; rm -rf ~"), "'build; rm -rf ~'");
    }

    #[test]
    fn 빈_문자열은_빈_따옴표_쌍이_된다() {
        assert_eq!(posix_quote(""), "''");
    }

    #[test]
    fn 공백이_포함된_이름도_하나의_단어로_보존된다() {
        assert_eq!(posix_quote("foo bar"), "'foo bar'");
    }

    #[test]
    fn 백슬래시는_이중으로_이스케이프된다() {
        assert_eq!(posix_quote("foo\\bar"), "'foo\\\\bar'");
    }

    #[test]
    fn 백슬래시_뒤_홑따옴표_조합은_fish_에서도_인용을_탈출하지_못하도록_이스케이프된다() {
        let quoted = posix_quote("foo\\'; touch CANARY_PWNED #");
        assert_eq!(quoted, "'foo\\\\'\\''; touch CANARY_PWNED #'");
    }
}
