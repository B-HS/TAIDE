use std::sync::Arc;

use lsp_types::{CompletionItemKind, Documentation, MarkupContent, MarkupKind};
use taide_native_editor::document::EditorError;
use taide_native_editor::json_completion::{self, Environment, Item};
use taide_native_editor::json_hover::{self, Hover};
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_schema_associations::{self, Pattern};
use taide_native_editor::json_schema_document::{self, Contribution};
use taide_native_editor::json_schema_evaluation;
use taide_native_editor::json_schema_graph;
use taide_native_editor::json_schema_load;
use taide_native_editor::json_schema_policy;
use taide_native_editor::json_schema_refs;
use taide_native_editor::json_schema_scalar;
use taide_native_editor::json_schema_service;
use taide_native_editor::json_schema_string;
use taide_native_editor::json_schema_uri;
use taide_native_editor::json_validation;
use taide_native_editor::json_value;

const SMALL: usize = 4096;
const WORK: usize = 1 << 24;
const PARSER: json_parser::Limits = json_parser::Limits {
    scanner: json_scanner::Limits {
        max_bytes: 1 << 20,
        max_tokens: 1 << 16,
        max_value_units: 1 << 20,
    },
    max_nodes: 1 << 16,
    max_depth: 512,
    max_diagnostics: SMALL,
    max_comments: SMALL,
};
const VALUES: json_value::Limits = json_value::Limits {
    parser: PARSER,
    max_values: 1 << 16,
    max_text_units: 1 << 20,
};
const URI: json_schema_uri::Limits = json_schema_uri::Limits {
    max_input_units: SMALL,
    max_output_units: 4 * SMALL,
    max_work: WORK,
};
const DOCUMENT: json_schema_document::Limits = json_schema_document::Limits {
    service: json_schema_service::Limits {
        associations: json_schema_associations::Limits {
            uri: URI,
            max_patterns: 64,
            max_pattern_units: SMALL,
            max_regex_units: 4 * SMALL,
            max_uris: 64,
            max_uri_units: 4 * SMALL,
            max_work: WORK,
        },
        max_handles: 64,
        max_registered: 64,
        max_associations: 64,
        max_values: 1 << 16,
        max_text_units: 1 << 20,
        max_work: WORK,
    },
    graph: json_schema_graph::Limits {
        refs: json_schema_refs::Limits {
            values: VALUES,
            max_work: WORK,
            max_merges: 1 << 16,
            max_text_units: 1 << 20,
            max_messages: SMALL,
            max_message_units: 1 << 20,
        },
        uri: URI,
        max_documents: 16,
        max_requests: 64,
        max_uri_units: 4 * SMALL,
    },
    load: json_schema_load::Limits {
        scanner: PARSER.scanner,
        uri: URI,
        max_values: 1 << 16,
        max_depth: 512,
        max_text_units: 1 << 20,
        max_work: WORK,
        max_messages: SMALL,
        max_message_units: 1 << 20,
    },
    policy: json_schema_policy::Limits {
        syntax: json_validation::Limits {
            parser: PARSER,
            max_diagnostics: SMALL,
            max_message_bytes: 1 << 20,
        },
        evaluation: json_schema_evaluation::Limits {
            values: VALUES,
            scalar: json_schema_scalar::Limits {
                max_work: WORK,
                max_diagnostics: SMALL,
                max_message_units: 1 << 20,
                max_enum_values: 1 << 16,
                max_depth: 512,
            },
            max_work: WORK,
            max_frames: SMALL,
            max_diagnostics: SMALL,
            max_message_units: 1 << 20,
            max_matches: 1 << 16,
            max_processed: 1 << 16,
        },
        max_schema_work: WORK,
    },
    contributions: VALUES,
    max_steps: 64,
};
const COMPLETION: json_completion::Limits = json_completion::Limits {
    document: DOCUMENT,
    max_items: SMALL,
    max_text_bytes: 1 << 20,
    max_work: WORK,
    max_depth: 512,
};
const HOVER: json_hover::Limits = json_hover::Limits {
    document: DOCUMENT,
    max_text_bytes: 1 << 20,
    max_work: WORK,
    max_depth: 512,
};
const SCHEMA_URI: &str = "http://example.com/test.json";
const SCHEMA: &str = r#"{
    "type": "object",
    "properties": {
        "name": {"type": "string", "title": "Name", "description": "a\r\nb\nc x.y"},
        "kind": {"enum": ["a", "b"], "enumDescriptions": ["first", "second"], "markdownDescription": "**Kind**"},
        "flag": {"type": "boolean"},
        "count": {"type": "integer", "default": 3},
        "hidden": {"type": "string", "doNotSuggest": true},
        "items": {"type": "array", "items": {"type": "string", "enum": ["x", "y"]}, "uniqueItems": true},
        "nested": {"properties": {"a": {}}}
    }
}"#;

struct NoAssociations;
impl json_schema_associations::PatternCompiler for NoAssociations {
    fn compile(&self, _: &[u16]) -> Result<Box<dyn Pattern>, EditorError> {
        Err(EditorError::Refused)
    }
}

struct NoPatterns;
impl json_schema_string::PatternCompiler for NoPatterns {
    fn compile(
        &self,
        _: &[u16],
        _: bool,
        _: bool,
    ) -> Result<Option<Box<dyn Pattern>>, EditorError> {
        Ok(None)
    }
}

fn contributions() -> Vec<Contribution> {
    let mut contributions = json_schema_document::contributions(VALUES).unwrap();
    contributions.push(Contribution {
        uri: SCHEMA_URI.encode_utf16().collect(),
        schema: Arc::new(json_value::parse(SCHEMA, VALUES).unwrap()),
    });
    contributions
}

fn with_environment<T>(run: impl FnOnce(Environment<'_>) -> T) -> T {
    let contributions = contributions();
    run(Environment {
        resource: "file:///project/test.json",
        contributions: &contributions,
        associations: &NoAssociations,
        patterns: &NoPatterns,
        cancelled: &|| false,
    })
}

fn complete(input: &str, cursor: &str) -> json_completion::Completion {
    let byte = input.find(cursor).unwrap();
    let input = input.replacen(cursor, "", 1);
    with_environment(|environment| {
        json_completion::complete(&input, byte, environment, COMPLETION).unwrap()
    })
}

fn hover(input: &str, byte: usize) -> Option<Hover> {
    with_environment(|environment| json_hover::hover(input, byte, environment, HOVER).unwrap())
}

fn plain(text: &str) -> Option<Documentation> {
    Some(Documentation::String(text.into()))
}

fn item(
    label: &str,
    kind: CompletionItemKind,
    insert_text: &str,
    range: std::ops::Range<usize>,
) -> Item {
    Item {
        label: label.into(),
        kind,
        insert_text: insert_text.into(),
        filter_text: None,
        sort_text: None,
        documentation: plain(""),
        detail: None,
        command: None,
        range,
    }
}

fn property(label: &str, insert_text: &str, range: std::ops::Range<usize>) -> Item {
    Item {
        filter_text: Some(format!("\"{label}\"")),
        ..item(label, CompletionItemKind::PROPERTY, insert_text, range)
    }
}

fn suggest() -> Option<lsp_types::Command> {
    Some(lsp_types::Command {
        title: "Suggest".into(),
        command: "editor.action.triggerSuggest".into(),
        arguments: None,
    })
}

#[test]
fn json_schema없는_완성은_최상위_schema_속성과_비슷한_객체_속성과_입력_단어를_원본처럼_제안한다() {
    let top = complete("{|}", "|");
    assert_eq!(
        top.items,
        vec![property("$schema", "\"\\$schema\": $1", 1..1)]
    );
    assert!(!top.is_incomplete);

    let word = complete("{ab|}", "|");
    assert_eq!(
        word.items,
        vec![
            property("$schema", "\"\\$schema\": $1", 1..3),
            Item {
                filter_text: None,
                ..property("\"ab\"", "\"ab\"", 1..3)
            },
        ]
    );
    assert!(word.is_incomplete);

    let similar = complete("[{\"x\": 1, \"y\": 2}, {\"x\": 3, \"|\"}]", "|");
    assert_eq!(similar.items, vec![property("y", "\"y\"", 28..30)]);

    let dollar = complete("{\"$schema\":| }", "|");
    let draft = |version: &str| {
        let label = format!("\"http://json-schema.org/draft-{version}/schema#\"");
        Item {
            filter_text: Some(label.clone()),
            ..item(&label, CompletionItemKind::MODULE, &label, 11..11)
        }
    };
    let mut expected = vec![draft("04"), draft("07")];
    let test_schema = format!("\"{SCHEMA_URI}\"");
    expected.push(Item {
        filter_text: Some(test_schema.clone()),
        ..item(
            &test_schema,
            CompletionItemKind::MODULE,
            &test_schema,
            11..11,
        )
    });
    assert_eq!(dollar.items, expected);
}

#[test]
fn json_schema_속성_완성은_값_스니펫과_suggest_재요청과_문서를_원본_규칙대로_만든다() {
    let input = format!("{{\"$schema\": \"{SCHEMA_URI}\", |}}");
    let byte = input.find('|').unwrap();
    let result = complete(&input, "|");
    let at = byte..byte;
    assert_eq!(
        result.items,
        vec![
            Item {
                documentation: plain("a\r\nb\nc x.y"),
                ..property("name", "\"name\": \"$1\"", at.clone())
            },
            Item {
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "**Kind**".into(),
                })),
                command: suggest(),
                ..property("kind", "\"kind\": $1", at.clone())
            },
            Item {
                command: suggest(),
                ..property("flag", "\"flag\": $1", at.clone())
            },
            property("count", "\"count\": ${1:3}", at.clone()),
            property("items", "\"items\": [$1]", at.clone()),
            property("nested", "\"nested\": {$1}", at),
        ]
    );
    assert!(!result.is_incomplete);
}

#[test]
fn json_schema_값_완성은_enum_설명과_boolean과_unique_items와_빈_배열_채움을_원본처럼_제안한다() {
    let prefix = format!("{{\"$schema\": \"{SCHEMA_URI}\", ");
    let kind = complete(&format!("{prefix}\"kind\":| }}"), "|");
    let at = prefix.len() + 7..prefix.len() + 7;
    assert_eq!(
        kind.items,
        vec![
            Item {
                documentation: plain("first"),
                ..item("\"a\"", CompletionItemKind::VALUE, "\"a\"", at.clone())
            },
            Item {
                documentation: plain("second"),
                ..item("\"b\"", CompletionItemKind::VALUE, "\"b\"", at.clone())
            },
        ]
    );

    let flag = complete(&format!("{prefix}\"flag\":| }}"), "|");
    assert_eq!(
        flag.items,
        vec![
            item("true", CompletionItemKind::VALUE, "true", at.clone()),
            item("false", CompletionItemKind::VALUE, "false", at.clone()),
        ]
    );

    let unique = complete(&format!("{prefix}\"items\": [\"x\", |]}}"), "|");
    let byte = prefix.len() + "\"items\": [\"x\", ".len();
    assert_eq!(
        unique.items,
        vec![Item {
            documentation: None,
            ..item("\"y\"", CompletionItemKind::VALUE, "\"y\"", byte..byte)
        }]
    );

    let filler = complete(&format!("{prefix}\"items\":| }}"), "|");
    let byte = prefix.len() + 8;
    assert_eq!(
        filler.items,
        vec![Item {
            detail: Some("New array".into()),
            ..item("[]", CompletionItemKind::VALUE, "[$1]", byte..byte)
        }]
    );
}

#[test]
fn json_완성은_주석_안과_여는_중괄호_앞과_잘못된_경계에서_제안하지_않는다() {
    assert!(complete("{ // a|\n}", "|").items.is_empty());
    assert!(complete("|{}", "|").items.is_empty());
    let error = with_environment(|environment| {
        json_completion::complete("{\"é\"}", 3, environment, COMPLETION)
    });
    assert_eq!(error, Err(EditorError::InvalidBoundary));
}

#[test]
fn json_hover는_키에서_값_schema의_제목과_설명과_enum_설명을_원본_markdown으로_합친다() {
    let prefix = format!("{{\"$schema\": \"{SCHEMA_URI}\", ");
    let kind = format!("{prefix}\"kind\": \"b\"}}");
    let key = prefix.len()..prefix.len() + "\"kind\"".len();
    assert_eq!(
        hover(&kind, key.start + 1),
        Some(Hover {
            contents: "**Kind**\n\n`b`: second".into(),
            range: key,
        })
    );

    let name = format!("{prefix}\"name\": \"x\"}}");
    let value = prefix.len() + "\"name\": ".len()..name.len() - 1;
    assert_eq!(
        hover(&name, value.start),
        Some(Hover {
            contents: "Name\n\na\n\nb\nc x\\.y".into(),
            range: value,
        })
    );

    assert_eq!(hover(&name, prefix.len() - 1), None);
    let no_schema = "{\"name\": \"x\"}";
    assert_eq!(hover(no_schema, 2), None);
    let missing_value = format!("{prefix}\"kind\"}}");
    assert_eq!(hover(&missing_value, prefix.len() + 1), None);
}
