use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_native_editor::diagnostics::{Marker, MarkerSet, Severity};
use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_validation::{Level, Limits, Settings, syntax_diagnostics};
use taide_native_editor::store::{EditorLimits, EditorStore};

const LIMITS: Limits = Limits {
    parser: json_parser::Limits {
        scanner: json_scanner::Limits {
            max_bytes: 64 * 1024,
            max_tokens: 128 * 1024,
            max_value_units: 64 * 1024,
        },
        max_nodes: 32 * 1024,
        max_depth: 16 * 1024,
        max_diagnostics: 32 * 1024,
        max_comments: 32 * 1024,
    },
    max_diagnostics: 32 * 1024,
    max_message_bytes: 2 * 1024 * 1024,
};
const SETTINGS: Settings = Settings {
    validate: true,
    allow_comments: true,
    comments: Some(Level::Error),
    trailing_commas: Some(Level::Error),
    schema_allows_comments: false,
    schema_allows_trailing_commas: false,
};
const SAMPLE_COUNT: usize = 8;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 8;

include!("fixtures/json-validation-oracle.rs");

fn units(value: &str) -> String {
    value
        .encode_utf16()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn describe(input: &str, markers: &[Marker]) -> String {
    markers
        .iter()
        .map(|marker| {
            let message = &marker.message;
            let severity = match message.severity {
                Severity::Error => 1,
                Severity::Warning => 2,
                _ => panic!("Unexpected syntax severity"),
            };
            let start = input[..marker.bytes.start].encode_utf16().count();
            let end = input[..marker.bytes.end].encode_utf16().count();
            let source = message.source.as_deref().map_or("-".to_owned(), units);
            format!(
                "{}:{severity}:{start}..{end}:{}:{source};",
                message.code.as_deref().unwrap(),
                units(&message.text)
            )
        })
        .collect()
}

#[test]
fn json_validation은_원본의_enable_주석_쉼표_심각도_메시지와_schema허용을_보존한다() {
    let mut mismatches = Vec::new();
    for &(
        input,
        validate,
        allow_comments,
        comments,
        trailing_commas,
        schema_allows_comments,
        schema_allows_trailing_commas,
        raw,
        expected,
    ) in CASES
    {
        let settings = Settings {
            validate,
            allow_comments,
            comments,
            trailing_commas,
            schema_allows_comments,
            schema_allows_trailing_commas,
        };
        let actual = syntax_diagnostics(input, "json", settings, LIMITS)
            .map(|markers| describe(input, &markers));
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!(
                "{input:?}/{settings:?}: actual={actual:?}, expected={expected:?}, source={raw:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(SAMPLE_COUNT)].join("\n")
    );
}

#[test]
fn json_validation의_marker는_실제문서에_붙고_내용과_revision을_보존한다() {
    let input = "{/*x*/\"𐐀\":1,\"𐐀\":2,}";
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: LIMITS.parser.scanner.max_bytes,
    })
    .unwrap();
    let path = "/synthetic/builtin-validation.json";
    let document = store
        .open_file(
            path.into(),
            OpenedFile {
                path: path.into(),
                content: input.into(),
                language_id: "json".into(),
                byte_size: u32::try_from(input.len()).unwrap(),
                line_count: 1,
                tier: FileSizeTier::Normal,
                read_only: true,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let snapshot = store.documents().snapshot(document).unwrap();
    let markers = syntax_diagnostics(input, "json", SETTINGS, LIMITS).unwrap();
    assert_eq!(markers.len(), 4);
    for marker in &markers {
        assert!(
            input.is_char_boundary(marker.bytes.start) && input.is_char_boundary(marker.bytes.end)
        );
        assert!(marker.bytes.start <= marker.bytes.end && marker.bytes.end <= input.len());
    }
    let set = MarkerSet::new(&snapshot, markers).unwrap();
    assert_eq!(set.revision(), snapshot.revision);
    assert_eq!(set.markers().len(), 4);
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.rope.to_string(), input);
    assert_eq!(after.revision, snapshot.revision);
    assert!(!after.dirty && after.metadata.read_only);
}

#[test]
fn json_validation은_전체진단_메시지_파서예산과_비활성화를_검증한다() {
    assert!(matches!(
        syntax_diagnostics(
            "[0,]",
            "json",
            SETTINGS,
            Limits {
                max_diagnostics: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        syntax_diagnostics(
            "/*x*/{}",
            "json",
            SETTINGS,
            Limits {
                max_message_bytes: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        syntax_diagnostics(
            "[0,]",
            "json",
            SETTINGS,
            Limits {
                parser: json_parser::Limits {
                    max_diagnostics: 0,
                    ..LIMITS.parser
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        syntax_diagnostics(
            "{}",
            "json",
            SETTINGS,
            Limits {
                max_diagnostics: 0,
                max_message_bytes: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .is_empty()
    );
    assert!(
        syntax_diagnostics(
            "unknown",
            "json",
            Settings {
                validate: false,
                ..SETTINGS
            },
            Limits {
                max_diagnostics: 0,
                max_message_bytes: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .is_empty()
    );
    let markers = syntax_diagnostics(
        "/*x*/{\"a\":1,}",
        "json",
        Settings {
            comments: Some(Level::Ignore),
            trailing_commas: Some(Level::Ignore),
            ..SETTINGS
        },
        LIMITS,
    )
    .unwrap();
    assert!(markers.is_empty());
    let markers = syntax_diagnostics(
        "/*x*/{\"a\":1,}",
        "json",
        Settings {
            comments: Some(Level::Warning),
            trailing_commas: Some(Level::Warning),
            ..SETTINGS
        },
        LIMITS,
    )
    .unwrap();
    assert!(
        markers
            .iter()
            .all(|marker| marker.message.severity == Severity::Warning)
    );
    assert_eq!(markers.len(), 2);
}
