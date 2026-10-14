use std::sync::Arc;

use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_associations::{Pattern, PatternCompiler};
use taide_native_editor::json_schema_refs::resolve_arena;
use taide_native_editor::json_schema_service::{LoadState, ResolveState, Service, Unresolved};
use taide_native_editor::json_value;

include!("fixtures/json-schema-service-limits.rs");

struct MatchingCompiler;
struct MatchingPattern;

impl Pattern for MatchingPattern {
    fn is_match(&self, _: &[u16]) -> bool {
        true
    }
}
impl PatternCompiler for MatchingCompiler {
    fn compile(&self, _: &[u16]) -> Result<Box<dyn Pattern>, EditorError> {
        Ok(Box::new(MatchingPattern))
    }
}

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}
fn schema(value: &str) -> Arc<json_value::Arena> {
    Arc::new(json_value::parse(value, VALUE_LIMITS).unwrap())
}

#[test]
fn json_schema_service는_공유요청_캐시와_실제소유_generation을_지킨다() {
    let mut service = Service::new(&MatchingCompiler, LIMITS);
    let id = units("urn:a");
    service
        .register(&id, Some(schema("{\"title\":\"a\"}")), &[], None, false)
        .unwrap();
    let ResolveState::Start {
        ticket,
        source: LoadState::Ready(source),
    } = service.begin_resolve(&id).unwrap()
    else {
        panic!("missing source");
    };
    let ResolveState::Pending(repeated) = service.begin_resolve(&id).unwrap() else {
        panic!("missing shared request");
    };
    assert_eq!(ticket, repeated);
    let result = resolve_arena((*source.schema).clone(), "urn:a", REF_LIMITS).unwrap();
    service.publish_resolved(&ticket, result, &[]).unwrap();
    let ResolveState::Ready(first) = service.begin_resolve(&id).unwrap() else {
        panic!("missing cache");
    };
    let ResolveState::Ready(second) = service.begin_resolve(&id).unwrap() else {
        panic!("missing cache");
    };
    assert!(Arc::ptr_eq(&first, &second));
    assert!(service.on_resource_change(&units("urn:a#")).unwrap());
    let LoadState::Start(old) = service.begin_load(&id).unwrap() else {
        panic!("inline content was retained");
    };
    let LoadState::Pending(same) = service.begin_load(&id).unwrap() else {
        panic!("duplicate request");
    };
    assert_eq!(old, same);
    assert_eq!(service.request_uri(&old).unwrap(), id);
    assert!(service.on_resource_change(&id).unwrap());
    assert_eq!(
        service.publish_load(
            &old,
            Unresolved {
                schema: schema("{}"),
                errors: Vec::new()
            }
        ),
        Err(EditorError::StaleRevision)
    );
    let LoadState::Start(current) = service.begin_load(&id).unwrap() else {
        panic!("missing new request");
    };
    assert_ne!(old, current);
    let mut other = Service::new(&MatchingCompiler, LIMITS);
    let LoadState::Start(foreign) = other.begin_load(&id).unwrap() else {
        panic!("missing foreign request");
    };
    assert_eq!(
        service.publish_load(
            &foreign,
            Unresolved {
                schema: schema("{}"),
                errors: Vec::new()
            }
        ),
        Err(EditorError::StaleRevision)
    );
    service
        .publish_load(
            &current,
            Unresolved {
                schema: schema("false"),
                errors: vec![units("owned loader error")],
            },
        )
        .unwrap();
    let LoadState::Ready(loaded) = service.begin_load(&id).unwrap() else {
        panic!("missing load cache");
    };
    assert_eq!(loaded.errors, vec![units("owned loader error")]);
    service
        .register(&id, Some(schema("{}")), &[], None, false)
        .unwrap();
    assert_eq!(
        service.publish_load(
            &current,
            Unresolved {
                schema: schema("{}"),
                errors: Vec::new()
            }
        ),
        Err(EditorError::StaleRevision)
    );
    let mut pending_service = Service::new(&MatchingCompiler, LIMITS);
    let ResolveState::Start {
        ticket: resolve_ticket,
        source: LoadState::Start(load_ticket),
    } = pending_service.begin_resolve(&id).unwrap()
    else {
        panic!("missing coupled load");
    };
    assert_eq!(
        pending_service.publish_resolved(
            &resolve_ticket,
            resolve_arena((*schema("{}")).clone(), "urn:a", REF_LIMITS).unwrap(),
            &[]
        ),
        Err(EditorError::InvalidBoundary)
    );
    assert_eq!(
        pending_service.publish_load(
            &load_ticket,
            Unresolved {
                schema: schema("\"wrong shape\""),
                errors: Vec::new()
            }
        ),
        Err(EditorError::InvalidBoundary)
    );
    pending_service
        .publish_load(
            &load_ticket,
            Unresolved {
                schema: schema("{}"),
                errors: Vec::new(),
            },
        )
        .unwrap();
    pending_service
        .publish_resolved(
            &resolve_ticket,
            resolve_arena((*schema("{}")).clone(), "urn:a", REF_LIMITS).unwrap(),
            &[],
        )
        .unwrap();
}

#[test]
fn json_schema_service는_전이적의존을_만료하고_부분외부해결을_캐시하지않는다() {
    let mut service = Service::new(&MatchingCompiler, LIMITS);
    for (id, dependencies) in [
        ("urn:a", vec![units("urn:b")]),
        ("urn:b", vec![units("urn:c")]),
        ("urn:c", Vec::new()),
    ] {
        let id_units = units(id);
        service
            .register(&id_units, Some(schema("{}")), &[], None, false)
            .unwrap();
        let ResolveState::Start {
            ticket,
            source: LoadState::Ready(source),
        } = service.begin_resolve(&id_units).unwrap()
        else {
            panic!("missing source");
        };
        service
            .publish_resolved(
                &ticket,
                resolve_arena((*source.schema).clone(), id, REF_LIMITS).unwrap(),
                &dependencies,
            )
            .unwrap();
    }
    assert!(service.on_resource_change(&units("urn:c#")).unwrap());
    assert!(!service.on_resource_change(&units("urn:c")).unwrap());
    for id in ["urn:a", "urn:b", "urn:c"] {
        assert!(matches!(
            service.begin_load(&units(id)).unwrap(),
            LoadState::Start(_)
        ));
    }
    let id = units("urn:external");
    service
        .register(
            &id,
            Some(schema("{\"$ref\":\"https://host/schema\"}")),
            &[],
            None,
            false,
        )
        .unwrap();
    let ResolveState::Start {
        ticket,
        source: LoadState::Ready(source),
    } = service.begin_resolve(&id).unwrap()
    else {
        panic!("missing source");
    };
    let partial = resolve_arena((*source.schema).clone(), "urn:external", REF_LIMITS).unwrap();
    assert_eq!(
        service.publish_resolved(&ticket, partial, &[]),
        Err(EditorError::InvalidBoundary)
    );
    assert!(matches!(
        service.begin_resolve(&id).unwrap(),
        ResolveState::Pending(_)
    ));
}

#[test]
fn json_schema_service는_원자적교체_합성선택과_전체저장예산을_지킨다() {
    let id = units("urn:a");
    let mut strict = Service::new(
        &MatchingCompiler,
        taide_native_editor::json_schema_service::Limits {
            max_text_units: id.len(),
            ..LIMITS
        },
    );
    assert_eq!(
        strict.register(&id, None, &[], None, false),
        Err(EditorError::Capacity)
    );
    assert!(strict.registered_ids(|_| true).unwrap().is_empty());
    let mut service = Service::new(
        &MatchingCompiler,
        taide_native_editor::json_schema_service::Limits {
            max_handles: 1,
            max_registered: 1,
            ..LIMITS
        },
    );
    service
        .register(&id, Some(schema("{}")), &[], None, false)
        .unwrap();
    service
        .register(&id, Some(schema("false")), &[], None, false)
        .unwrap();
    assert_eq!(
        service.register(&units("urn:b"), Some(schema("{}")), &[], None, false),
        Err(EditorError::Capacity)
    );
    assert_eq!(service.registered_ids(|_| true).unwrap(), vec![id.clone()]);
    let LoadState::Ready(content) = service.begin_load(&id).unwrap() else {
        panic!("replacement lost");
    };
    assert!(matches!(
        content.schema.get(content.schema.root().unwrap()),
        Some(json_value::Value::Boolean(false))
    ));
    for limits in [
        taide_native_editor::json_schema_service::Limits {
            max_handles: 0,
            ..LIMITS
        },
        taide_native_editor::json_schema_service::Limits {
            max_registered: 0,
            ..LIMITS
        },
        taide_native_editor::json_schema_service::Limits {
            max_associations: 0,
            ..LIMITS
        },
        taide_native_editor::json_schema_service::Limits {
            max_values: 0,
            ..LIMITS
        },
        taide_native_editor::json_schema_service::Limits {
            max_text_units: 0,
            ..LIMITS
        },
    ] {
        let mut service = Service::new(&MatchingCompiler, limits);
        assert_eq!(
            service.register(&id, Some(schema("{}")), &[units("*.json")], None, false),
            Err(EditorError::Capacity)
        );
        assert!(service.registered_ids(|_| true).unwrap().is_empty());
        let mut contributed = Service::new(&MatchingCompiler, limits);
        assert_eq!(
            contributed.register(&id, Some(schema("{}")), &[units("*.json")], None, true),
            Err(EditorError::Capacity)
        );
    }
    let mut service = Service::new(&MatchingCompiler, LIMITS);
    for id in ["urn:a", "urn:b"] {
        service
            .register(
                &units(id),
                Some(schema("{}")),
                &[units("*.json")],
                None,
                false,
            )
            .unwrap();
    }
    let resource = units("file:///a'(!).json?q#f");
    let selected = service.select(&resource, None).unwrap().unwrap();
    assert_eq!(
        selected,
        units("schemaservice://combinedSchema/file%3A%2F%2F%2Fa'(!).json%3Fq%23f")
    );
    let LoadState::Ready(first) = service.begin_load(&selected).unwrap() else {
        panic!("combined content lost");
    };
    assert_eq!(
        service.select(&resource, None).unwrap(),
        Some(selected.clone())
    );
    let LoadState::Ready(second) = service.begin_load(&selected).unwrap() else {
        panic!("combined cache lost");
    };
    assert!(Arc::ptr_eq(&first, &second));
    service.on_resource_change(&units("urn:unrelated")).unwrap();
    assert_eq!(
        service.select(&resource, None).unwrap(),
        Some(selected.clone())
    );
    let LoadState::Ready(third) = service.begin_load(&selected).unwrap() else {
        panic!("combined refresh lost");
    };
    assert!(!Arc::ptr_eq(&first, &third));
}
