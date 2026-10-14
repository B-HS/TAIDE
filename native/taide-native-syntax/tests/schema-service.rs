use std::collections::VecDeque;
use std::sync::Arc;

use taide_native_editor::json_parser;
use taide_native_editor::json_schema_refs::resolve_arena;
use taide_native_editor::json_schema_service::{LoadState, ResolveState, Service, Unresolved};
use taide_native_editor::json_value::{self, Arena, Value};
use taide_native_syntax::JsonSchemaPatternCompiler;

include!("../../taide-native-editor/tests/fixtures/json-schema-service-limits.rs");

enum Action {
    Reg(
        &'static str,
        Option<&'static str>,
        &'static [&'static str],
        Option<&'static str>,
        bool,
    ),
    Clear,
    Ids,
    Uris(&'static str, Option<&'static str>),
    Combined(&'static str),
    Load(&'static str),
    Resolve(&'static str),
    Change(&'static str),
}

include!("../../taide-native-editor/tests/fixtures/json-schema-service-oracle.rs");

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}
fn hex(value: &[u16]) -> String {
    value
        .iter()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn list(value: &[Vec<u16>]) -> String {
    value
        .iter()
        .map(|value| hex(value))
        .collect::<Vec<_>>()
        .join(";")
}

fn describe(arena: &Arena) -> String {
    let mut pending = VecDeque::from([(arena.root().unwrap(), "-1".to_owned(), "-".to_owned())]);
    let mut parts = Vec::new();
    while let Some((index, parent, edge)) = pending.pop_front() {
        let cursor = parts.len();
        let mut content = String::new();
        let kind = match arena.get(index).unwrap() {
            Value::Object(members) => {
                for member in members {
                    pending.push_back((
                        member.value,
                        cursor.to_string(),
                        format!("k{}", hex(&member.key)),
                    ));
                }
                "object"
            }
            Value::Array(items) => {
                for (position, &item) in items.iter().enumerate() {
                    pending.push_back((item, cursor.to_string(), format!("i{position}")));
                }
                "array"
            }
            Value::Null => "null",
            Value::String(value) => {
                content = hex(value);
                "string"
            }
            Value::Number(value) => {
                content = value.to_string();
                "number"
            }
            Value::Boolean(value) => {
                content = value.to_string();
                "boolean"
            }
        };
        parts.push(format!("{kind}:{parent}:{edge}:{content};"));
    }
    parts.join("")
}

fn load(
    service: &mut Service<'_>,
    id: &[u16],
    state: LoadState,
    requests: &mut usize,
) -> Arc<Unresolved> {
    if let LoadState::Ready(source) = state {
        return source;
    }
    let LoadState::Start(ticket) = state else {
        panic!("unexpected unresolved request");
    };
    *requests += 1;
    let uri = String::from_utf16(service.request_uri(&ticket).unwrap()).unwrap();
    assert!(!uri.contains(['"', '\\']));
    let schema =
        json_value::parse(&format!("{{\"title\":\"loaded:{uri}\"}}"), VALUE_LIMITS).unwrap();
    service
        .publish_load(
            &ticket,
            Unresolved {
                schema: Arc::new(schema),
                errors: Vec::new(),
            },
        )
        .unwrap();
    let LoadState::Ready(source) = service.begin_load(id).unwrap() else {
        panic!("missing source");
    };
    source
}

#[test]
fn schema_service는_실제원본의_등록_선택_합성과_캐시흐름에_일치한다() {
    for &(actions, raw, expected) in CASES {
        let mut service = Service::new(&JsonSchemaPatternCompiler, LIMITS);
        let mut requests = 0;
        for (index, action) in actions.iter().enumerate() {
            let result = match action {
                Action::Reg(id, schema, patterns, folder, is_contribution) => {
                    let schema = schema
                        .map(|input| Arc::new(json_value::parse(input, VALUE_LIMITS).unwrap()));
                    let patterns = patterns
                        .iter()
                        .map(|pattern| units(pattern))
                        .collect::<Vec<_>>();
                    let folder = folder.map(units);
                    service
                        .register(
                            &units(id),
                            schema,
                            &patterns,
                            folder.as_deref(),
                            *is_contribution,
                        )
                        .unwrap();
                    "ok".to_owned()
                }
                Action::Clear => {
                    service.clear_external();
                    "ok".to_owned()
                }
                Action::Ids => format!("ids:{}", list(&service.registered_ids(|_| true).unwrap())),
                Action::Uris(resource, input) => {
                    let document = input
                        .map(|input| json_parser::parse(input, true, VALUE_LIMITS.parser).unwrap());
                    format!(
                        "uris:{}",
                        list(
                            &service
                                .schema_uris(&units(resource), document.as_ref())
                                .unwrap()
                        )
                    )
                }
                Action::Combined(resource) => {
                    let id = service.select(&units(resource), None).unwrap().unwrap();
                    let state = service.begin_load(&id).unwrap();
                    let source = load(&mut service, &id, state, &mut requests);
                    format!("combined:{}|{}", hex(&id), describe(&source.schema))
                }
                Action::Load(id) => {
                    let id = units(id);
                    let state = service.begin_load(&id).unwrap();
                    let source = load(&mut service, &id, state, &mut requests);
                    format!(
                        "load:{}|errors:{}|requests:{requests}",
                        describe(&source.schema),
                        source.errors.len()
                    )
                }
                Action::Resolve(id) => {
                    let id_units = units(id);
                    if let ResolveState::Start { ticket, source } =
                        service.begin_resolve(&id_units).unwrap()
                    {
                        let source = load(&mut service, &id_units, source, &mut requests);
                        let resolved =
                            resolve_arena((*source.schema).clone(), id, REF_LIMITS).unwrap();
                        service.publish_resolved(&ticket, resolved, &[]).unwrap();
                    }
                    let ResolveState::Ready(source) = service.begin_resolve(&id_units).unwrap()
                    else {
                        panic!("missing resolved cache");
                    };
                    format!(
                        "resolve:{}|errors:{}|requests:{requests}",
                        describe(&source.schema),
                        source.errors.len()
                    )
                }
                Action::Change(id) => {
                    format!("change:{}", service.on_resource_change(&units(id)).unwrap())
                }
            };
            assert_eq!(
                result, expected[index],
                "event {index}, source raw={}",
                raw[index]
            );
        }
    }
}

#[test]
fn schema_service는_실제consumer에서_명시schema가_파일연관을_우선한다() {
    let mut service = Service::new(&JsonSchemaPatternCompiler, LIMITS);
    service
        .register(
            &units("urn:a"),
            Some(Arc::new(json_value::parse("{}", VALUE_LIMITS).unwrap())),
            &[units("*.json")],
            None,
            false,
        )
        .unwrap();
    let resource = units("file:///root/document.json");
    assert_eq!(
        service.select(&resource, None).unwrap(),
        Some(units("urn:a"))
    );
    let document = json_parser::parse(
        "{\"$schema\":\"../schemas/custom.json\"}",
        true,
        VALUE_LIMITS.parser,
    )
    .unwrap();
    assert_eq!(
        service.select(&resource, Some(&document)).unwrap(),
        Some(units("file:///schemas/custom.json"))
    );
    assert_eq!(
        service.select(&resource, None).unwrap(),
        Some(units("urn:a"))
    );
    assert!(matches!(
        service
            .begin_load(&units("file:///schemas/custom.json"))
            .unwrap(),
        LoadState::Start(_)
    ));
}
