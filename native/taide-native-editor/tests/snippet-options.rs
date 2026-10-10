use taide_native_editor::snippet_syntax::{
    FinalTabstopOptions, Marker, ParseLimits, RegexMetadata, parse_complete,
};

const BYTE_LIMIT: usize = 4096;
const MARKER_LIMIT: usize = 32;
const NESTING_LIMIT: usize = 8;

#[test]
fn 스니펫_정규화는_검증한_정규식의_전체_동작옵션을_보존한다() {
    for (authored, expected) in [
        ("s", "s"),
        ("msu", "msu"),
        ("ygim", "igmy"),
        ("d", "d"),
        ("v", "v"),
        ("iug", "igu"),
    ] {
        let source = format!("${{1:${{name/(.+)/${{1:/upcase}}/{authored}}}}} $1");
        let markers = parse_complete(
            &source,
            ParseLimits {
                max_bytes: BYTE_LIMIT,
                max_markers: MARKER_LIMIT,
                max_nesting: NESTING_LIMIT,
            },
            FinalTabstopOptions {
                insert: false,
                enforce: false,
            },
            |source, flags| {
                Some(RegexMetadata {
                    source: source.to_owned(),
                    ignore_case: flags.contains('i'),
                    global: flags.contains('g'),
                })
            },
        )
        .unwrap();
        let Marker::Placeholder { children, .. } = &markers[2] else {
            panic!("expected copied placeholder");
        };
        let Marker::Variable {
            transform: Some(transform),
            ..
        } = &children[0]
        else {
            panic!("expected variable transform");
        };
        assert_eq!(transform.options, expected, "authored flags {authored}");
    }
}
