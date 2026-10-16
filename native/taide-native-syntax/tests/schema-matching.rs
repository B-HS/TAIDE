use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser::{self, Kind};
use taide_native_editor::json_scanner;
use taide_native_editor::json_schema_evaluation::{
    self, Limits, MatchingSchema, SchemaView, Selection,
};
use taide_native_editor::json_schema_refs;
use taide_native_editor::json_schema_scalar;
use taide_native_editor::json_value::{self, Arena};
use taide_native_editor::json_value_text;
use taide_native_syntax::JsonSchemaValidationPatternCompiler;

const VALUES: json_value::Limits = json_value::Limits {
    parser: json_parser::Limits {
        scanner: json_scanner::Limits {
            max_bytes: 2 * 1024 * 1024,
            max_tokens: 4 * 1024 * 1024,
            max_value_units: 2 * 1024 * 1024,
        },
        max_nodes: 128 * 1024,
        max_depth: 64 * 1024,
        max_diagnostics: 32 * 1024,
        max_comments: 32 * 1024,
    },
    max_values: 128 * 1024,
    max_text_units: 4 * 1024 * 1024,
};
const LIMITS: Limits = Limits {
    values: VALUES,
    scalar: json_schema_scalar::Limits {
        max_work: 32 * 1024 * 1024,
        max_diagnostics: 8192,
        max_message_units: 4 * 1024 * 1024,
        max_enum_values: 32 * 1024,
        max_depth: 32 * 1024,
    },
    max_work: 32 * 1024 * 1024,
    max_frames: 8192,
    max_diagnostics: 8192,
    max_message_units: 4 * 1024 * 1024,
    max_matches: 128 * 1024,
    max_processed: 128 * 1024,
};
const TEXT: json_value_text::Limits = json_value_text::Limits {
    max_units: 4 * 1024 * 1024,
    max_work: 32 * 1024 * 1024,
    max_depth: 32 * 1024,
};
const NO_SELECTION: isize = -1;
const SAMPLE_COUNT: usize = 5;
const DEEP_NESTING: usize = 2048;

include!("../../taide-native-editor/tests/fixtures/json-schema-matching-oracle.rs");

fn hex(value: &[u16]) -> String {
    value
        .iter()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Object => "object",
        Kind::Array => "array",
        Kind::Property => "property",
        Kind::String => "string",
        Kind::Number => "number",
        Kind::Boolean => "boolean",
        Kind::Null => "null",
    }
}

fn byte_at(input: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (byte, character) in input.char_indices() {
        if units >= utf16 {
            return byte;
        }
        units += character.len_utf16();
    }
    input.len()
}

fn describe(
    document: &json_parser::Document,
    schema: &Arena,
    matches: &[MatchingSchema],
) -> String {
    matches
        .iter()
        .map(|matching| {
            let node = &document.nodes[matching.node];
            let view = match matching.view {
                SchemaView::Source => format!(
                    "S:{}",
                    hex(&json_value_text::stringify(schema, matching.schema, TEXT).unwrap())
                ),
                SchemaView::Empty => "E".into(),
                SchemaView::Reject => "R".into(),
            };
            format!(
                "{}..{}:{}:{view};",
                node.utf16.start, node.utf16.end, matching.inverted
            )
        })
        .collect()
}

fn matching(
    input: &str,
    schema: &str,
    focus: isize,
    exclude: isize,
    limits: Limits,
) -> Result<(json_parser::Document, Arena, Vec<MatchingSchema>, String), EditorError> {
    let document = json_parser::parse(input, false, VALUES.parser)?;
    let schema = json_value::parse(schema, VALUES)?;
    let (exclude_node, excluded) = if exclude == NO_SELECTION {
        (None, "-".to_owned())
    } else {
        let node = document.node_at_utf16(exclude as usize, false);
        let text = node.map_or("-".to_owned(), |index| {
            let node = &document.nodes[index];
            format!(
                "{}@{}..{}",
                kind(node.kind),
                node.utf16.start,
                node.utf16.end
            )
        });
        (node, text)
    };
    let selection = Selection {
        focus_byte: (focus != NO_SELECTION).then(|| byte_at(input, focus as usize)),
        exclude_node,
    };
    let matches = json_schema_evaluation::matching_schemas(
        &document,
        &schema,
        schema.root().ok_or(EditorError::InvalidIdentity)?,
        &JsonSchemaValidationPatternCompiler,
        selection,
        limits,
    )?;
    Ok((document, schema, matches, excluded))
}

#[test]
fn schema_matching은_실제원본의_focus_exclude_분기수집과_boolean임시표현을_보존한다() {
    let mut failures = Vec::new();
    for &(input, schema, focus, exclude, expected_exclude, expected) in CASES {
        let (document, arena, matches, excluded) =
            matching(input, schema, focus, exclude, LIMITS).unwrap();
        let actual = describe(&document, &arena, &matches);
        if excluded != expected_exclude || actual != expected {
            failures.push(format!(
                "{input} | {schema} | focus {focus} | exclude {exclude}\nexpected {expected_exclude} {expected}\nactual   {excluded} {actual}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} matching cases differ:\n{}",
        failures.len(),
        CASES.len(),
        failures
            .iter()
            .take(SAMPLE_COUNT)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn schema_matching은_범위밖_exclude와_예산과_깊은문서를_검증한다() {
    let document = json_parser::parse("[1]", false, VALUES.parser).unwrap();
    let schema = json_value::parse("{}", VALUES).unwrap();
    assert_eq!(
        json_schema_evaluation::matching_schemas(
            &document,
            &schema,
            schema.root().unwrap(),
            &JsonSchemaValidationPatternCompiler,
            Selection {
                focus_byte: None,
                exclude_node: Some(document.nodes.len()),
            },
            LIMITS,
        ),
        Err(EditorError::InvalidIdentity)
    );
    let tight = Limits {
        max_matches: 1,
        ..LIMITS
    };
    assert_eq!(
        matching("[1]", r#"{"items":{}}"#, NO_SELECTION, NO_SELECTION, tight).map(|_| ()),
        Err(EditorError::Capacity)
    );
    let focused = matching("[1]", r#"{"items":{}}"#, 0, NO_SELECTION, tight).unwrap();
    assert_eq!(focused.2.len(), 1);
    let deep = format!("{}{}", "[".repeat(DEEP_NESTING), "]".repeat(DEEP_NESTING));
    let refs = json_schema_refs::Limits {
        values: VALUES,
        max_work: LIMITS.max_work,
        max_merges: 32 * 1024,
        max_text_units: LIMITS.max_message_units,
        max_messages: LIMITS.max_diagnostics,
        max_message_units: LIMITS.max_message_units,
    };
    let resolved =
        json_schema_refs::resolve_local(r##"{"items":{"$ref":"#"}}"##, "file:///schema.json", refs)
            .unwrap();
    let document = json_parser::parse(&deep, false, VALUES.parser).unwrap();
    let all = json_schema_evaluation::matching_schemas(
        &document,
        &resolved.schema,
        resolved.schema.root().unwrap(),
        &JsonSchemaValidationPatternCompiler,
        Selection::default(),
        LIMITS,
    )
    .unwrap();
    assert_eq!(all.len(), DEEP_NESTING);
    let innermost = json_schema_evaluation::matching_schemas(
        &document,
        &resolved.schema,
        resolved.schema.root().unwrap(),
        &JsonSchemaValidationPatternCompiler,
        Selection {
            focus_byte: Some(DEEP_NESTING - 1),
            exclude_node: None,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(innermost.len(), DEEP_NESTING);
    let outer_only = json_schema_evaluation::matching_schemas(
        &document,
        &resolved.schema,
        resolved.schema.root().unwrap(),
        &JsonSchemaValidationPatternCompiler,
        Selection {
            focus_byte: Some(0),
            exclude_node: None,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(outer_only.len(), 1);
}
