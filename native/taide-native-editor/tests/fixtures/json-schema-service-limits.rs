const URI_LIMITS: taide_native_editor::json_schema_uri::Limits =
    taide_native_editor::json_schema_uri::Limits {
        max_input_units: 64 * 1024,
        max_output_units: 256 * 1024,
        max_work: 1024 * 1024,
    };
const VALUE_LIMITS: taide_native_editor::json_value::Limits =
    taide_native_editor::json_value::Limits {
        parser: taide_native_editor::json_parser::Limits {
            scanner: taide_native_editor::json_scanner::Limits {
                max_bytes: 64 * 1024,
                max_tokens: 128 * 1024,
                max_value_units: 64 * 1024,
            },
            max_nodes: 32 * 1024,
            max_depth: 16 * 1024,
            max_diagnostics: 32 * 1024,
            max_comments: 32 * 1024,
        },
        max_values: 16 * 1024,
        max_text_units: 128 * 1024,
    };
const REF_LIMITS: taide_native_editor::json_schema_refs::Limits =
    taide_native_editor::json_schema_refs::Limits {
        values: VALUE_LIMITS,
        max_work: 1024 * 1024,
        max_merges: 16 * 1024,
        max_text_units: 1024 * 1024,
        max_messages: 16 * 1024,
        max_message_units: 1024 * 1024,
    };
const LIMITS: taide_native_editor::json_schema_service::Limits =
    taide_native_editor::json_schema_service::Limits {
        associations: taide_native_editor::json_schema_associations::Limits {
            uri: URI_LIMITS,
            max_patterns: 1024,
            max_pattern_units: 64 * 1024,
            max_regex_units: 256 * 1024,
            max_uris: 1024,
            max_uri_units: 256 * 1024,
            max_work: 1024 * 1024,
        },
        max_handles: 1024,
        max_registered: 1024,
        max_associations: 1024,
        max_values: 128 * 1024,
        max_text_units: 1024 * 1024,
        max_work: 1024 * 1024,
    };
