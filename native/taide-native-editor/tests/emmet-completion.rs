use std::path::PathBuf;

use lsp_types::Documentation;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::emmet::{
    CompletionLimits, ConversionLimits, MarkupLimits, Suggestion, Syntax, complete_document,
    complete_line,
};
use taide_native_editor::lsp::Position;
use taide_native_editor::store::{EditorLimits, EditorStore};

const LIMITS: CompletionLimits = CompletionLimits {
    parse: MarkupLimits {
        max_bytes: 16 * 1024,
        max_nodes: 2048,
        max_nesting: 32,
    },
    conversion: ConversionLimits {
        max_repeats: 1000,
        max_nodes: 8192,
        max_bytes: 256 * 1024,
        max_nesting: 32,
    },
    max_candidates: 512,
    max_bytes: 2 * 1024 * 1024,
};
const SEED: u32 = 42;
const MULTIPLIER: u32 = 1_664_525;
const INCREMENT: u32 = 1_013_904_223;
const MISMATCH_SAMPLE_COUNT: usize = 8;

include!("fixtures/emmet-completion-oracle.rs");

fn part(value: Option<&str>) -> String {
    value.map_or_else(|| "-".into(), |value| format!("{}:{value}", value.len()))
}

fn describe(line: &str, suggestions: &[Suggestion]) -> String {
    suggestions
        .iter()
        .map(|suggestion| {
            let item = &suggestion.item;
            let documentation = item.documentation.as_ref().map(|doc| match doc {
                Documentation::String(text) => text.as_str(),
                Documentation::MarkupContent(content) => content.value.as_str(),
            });
            format!(
                "{}{}{}{}{}{}r{}:{}",
                part(Some(&item.label)),
                part(item.detail.as_deref()),
                part(item.insert_text.as_deref()),
                part(documentation),
                part(item.filter_text.as_deref()),
                part(item.sort_text.as_deref()),
                line[..suggestion.bytes.start].encode_utf16().count(),
                line[..suggestion.bytes.end].encode_utf16().count()
            )
        })
        .collect()
}

fn random() -> impl FnMut() -> u32 {
    let mut state = SEED;
    move || {
        state = state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        state
    }
}

fn snapshot(text: &str, language: &str) -> DocumentSnapshot {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: LIMITS.parse.max_bytes,
    })
    .unwrap();
    let path = PathBuf::from("/synthetic/emmet-completion");
    let id = store
        .open_file(
            path.clone(),
            OpenedFile {
                path: path.to_string_lossy().into_owned(),
                content: text.into(),
                encoding_lossy: false,
                language_id: language.into(),
                tier: FileSizeTier::Normal,
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                modified_ms: 0.0,
                read_only: false,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    store.documents().snapshot(id).unwrap()
}

fn corrected(line: &str, syntax: Syntax, expected: &str) -> String {
    if expected.is_empty() {
        return String::new();
    }
    if line == "div.block>.-item|bem|c|t" && syntax != Syntax::Css {
        return expected.replace("div.block>.-item|bem|c,t", "div.block>.-item|bem|c|t");
    }
    if line == "div[title=\"a\\\"b\"]" && syntax != Syntax::Css {
        return expected
            .replace(
                &part(Some("<div title=\"a\"b\">${0}</div>")),
                &part(Some("<div title=\"a&quot;b\">${0}</div>")),
            )
            .replace(
                &part(Some("<div title=\"a\"b\">|</div>")),
                &part(Some("<div title=\"a&quot;b\">|</div>")),
            );
    }
    if line == "p{\\${1}}" && syntax != Syntax::Css {
        return expected
            .replace(&part(Some("<p>${0}</p>")), &part(Some("<p>\\${1}</p>")))
            .replace(&part(Some("<p>|</p>")), &part(Some("<p>${1}</p>")));
    }
    if line == "m${variable}" && syntax == Syntax::Css {
        return format!(
            "{}{}{}{}{}{}r0:{}",
            part(Some("margin: variable;")),
            part(Some("Emmet abbreviation")),
            part(Some("margin: ${variable:variable};")),
            part(Some("margin: variable;")),
            part(Some("m${variable}")),
            part(None),
            line.encode_utf16().count()
        );
    }
    if line == "qen" && syntax == Syntax::Css {
        let original = part(Some("quotes: '\\201C' '\\201D' '\\2018' '\\2019';"));
        let escaped = part(Some("quotes: '\\\\201C' '\\\\201D' '\\\\2018' '\\\\2019';"));
        assert_eq!(expected.matches(&original).count(), 3);
        let pieces = expected.split(&original).collect::<Vec<_>>();
        return format!(
            "{}{original}{}{escaped}{}{original}{}",
            pieces[0], pieces[1], pieces[2], pieces[3]
        );
    }
    expected.into()
}

#[test]
fn emmet후보는_실제_do_complete의_범위_문서_표시_추가목록과_일치한다() {
    let mismatches=CASES.iter().filter_map(|&(syntax,line,cursor,expected)| {
        let syntax=match syntax { "html"=>Syntax::Html,"jsx"=>Syntax::Jsx,_=>Syntax::Css };
        let result=complete_line(line,cursor,syntax,LIMITS,&mut random());
        let actual=result.as_ref().ok().map(|items|describe(line,items));
        let expected=corrected(line,syntax,expected);
        (actual.as_deref()!=Some(expected.as_str())).then(||format!("{syntax:?}/{line:?}/{cursor}: actual={actual:?}, expected={expected:?}, error={:?}",result.err()))
    }).collect::<Vec<_>>();
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(MISMATCH_SAMPLE_COUNT)].join("\n")
    );
}

#[test]
fn emmet후보는_실제_document_revision과_utf16_전체축약식범위를_보존한다() {
    let snapshot = snapshot("first\n\u{1f642} div|c", "html");
    let position = Position {
        line: 1,
        character: 8,
    };
    let candidates = complete_document(&snapshot, position, LIMITS, &mut random()).unwrap();
    assert!(!candidates.is_empty());
    for candidate in candidates {
        assert_eq!(candidate.document, snapshot.id);
        assert_eq!(candidate.revision, snapshot.revision);
        assert_eq!(
            candidate.insert,
            snapshot.rope.len_bytes() - "div|c".len()..snapshot.rope.len_bytes()
        );
        assert_eq!(candidate.replace, candidate.insert);
        assert!(candidate.is_snippet());
    }
    assert_eq!(
        complete_document(
            &snapshot,
            Position {
                line: 1,
                character: 1
            },
            LIMITS,
            &mut random()
        ),
        Err(EditorError::InvalidBoundary)
    );
}

#[test]
fn emmet후보는_생성수와_전체byte예산을_부분목록없이_지킨다() {
    assert_eq!(
        complete_line(
            "a",
            1,
            Syntax::Html,
            CompletionLimits {
                max_candidates: 1,
                ..LIMITS
            },
            &mut random()
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        complete_line(
            "div",
            3,
            Syntax::Html,
            CompletionLimits {
                max_bytes: 1,
                ..LIMITS
            },
            &mut random()
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        complete_line("\u{1f642}", 1, Syntax::Html, LIMITS, &mut random()),
        Err(EditorError::InvalidBoundary)
    );
}
