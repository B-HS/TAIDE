use std::collections::{BTreeMap, VecDeque};

use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_associations::{Pattern, PatternCompiler};
use taide_native_editor::json_schema_graph::{self, Job, Progress, Start};
use taide_native_editor::json_schema_load::{self, Response};
use taide_native_editor::json_schema_service::{LoadState, ResolveState, Service};
use taide_native_editor::json_value::Value;

include!("fixtures/json-schema-service-limits.rs");
const GRAPH_LIMITS: json_schema_graph::Limits = json_schema_graph::Limits {
    refs: REF_LIMITS,
    uri: URI_LIMITS,
    max_documents: 128,
    max_requests: 4096,
    max_uri_units: 1024 * 1024,
};
const LOAD_LIMITS: json_schema_load::Limits = json_schema_load::Limits {
    scanner: VALUE_LIMITS.parser.scanner,
    uri: URI_LIMITS,
    max_values: VALUE_LIMITS.max_values,
    max_depth: VALUE_LIMITS.parser.max_depth,
    max_text_units: VALUE_LIMITS.max_text_units,
    max_work: 1024 * 1024,
    max_messages: 16,
    max_message_units: 1024 * 1024,
};
const MAX_STEPS: usize = 1024;

include!("fixtures/json-schema-graph-oracle.rs");

struct UnusedCompiler;
impl PatternCompiler for UnusedCompiler {
    fn compile(&self, _: &[u16]) -> Result<Box<dyn Pattern>, EditorError> {
        Err(EditorError::InvalidBoundary)
    }
}

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

fn describe(result: &taide_native_editor::json_schema_refs::Resolved) -> String {
    let arena = &result.schema;
    let mut queue = VecDeque::from([(arena.root().unwrap(), "-1".to_owned(), "-".to_owned())]);
    let mut seen = BTreeMap::new();
    let mut parts = Vec::new();
    while let Some((index, parent, edge)) = queue.pop_front() {
        let value = arena.get(index).unwrap();
        let cursor = parts.len();
        if matches!(value, Value::Object(_) | Value::Array(_)) {
            if let Some(first) = seen.get(&index) {
                parts.push(format!("ref:{parent}:{edge}:{first};"));
                continue;
            }
            seen.insert(index, cursor);
        }
        let mut content = String::new();
        let kind = match value {
            Value::Null => "null",
            Value::Boolean(value) => {
                content = value.to_string();
                "boolean"
            }
            Value::Number(value) => {
                content = value.to_string();
                "number"
            }
            Value::String(value) => {
                content = hex(value);
                "string"
            }
            Value::Object(members) => {
                for member in members {
                    queue.push_back((
                        member.value,
                        cursor.to_string(),
                        format!("k{}", hex(&member.key)),
                    ));
                }
                "object"
            }
            Value::Array(items) => {
                for (position, &item) in items.iter().enumerate() {
                    queue.push_back((item, cursor.to_string(), format!("i{position}")));
                }
                "array"
            }
        };
        parts.push(format!("{kind}:{parent}:{edge}:{content};"));
    }
    let errors = result
        .errors
        .iter()
        .map(|error| hex(error))
        .collect::<Vec<_>>()
        .join(";");
    let warning = if result.unsupported.is_empty() {
        String::new()
    } else {
        hex(&units(&format!(
            "The schema uses meta-schema features ({}) that are not yet supported by the validator.",
            result.unsupported.join(", ")
        )))
    };
    let draft = result.schema_draft.as_deref().map_or("-".to_owned(), hex);
    format!("{}|{errors}|{warning}|{draft}", parts.join(""))
}

#[test]
fn json_schema_graph는_실제원본의_외부참조_공유_순환_문맥과_진단에_일치한다() {
    for &(root, documents, workspace, raw, expected) in CASES {
        let mut service = Service::new(&UnusedCompiler, LIMITS);
        let Start::Job(mut job) =
            Job::start(&mut service, &units(root), workspace, GRAPH_LIMITS).unwrap()
        else {
            panic!("missing root job");
        };
        let mut calls = BTreeMap::new();
        let mut actual = None;
        for _ in 0..MAX_STEPS {
            match job.advance(&mut service).unwrap() {
                Progress::Ready(resolved) => {
                    actual = Some(describe(&resolved));
                    break;
                }
                Progress::Waiting {
                    job: next,
                    requests,
                } => {
                    job = next;
                    assert!(
                        !requests.is_empty(),
                        "unexpected externally pending fixture"
                    );
                    for request in requests {
                        let uri = json_schema_load::request_url(&request.uri, URI_LIMITS).unwrap();
                        let uri = String::from_utf16(&uri).unwrap();
                        *calls.entry(uri.clone()).or_insert(0usize) += 1;
                        if let Some((_, content)) = documents.iter().find(|(id, _)| *id == uri) {
                            json_schema_load::publish_response(
                                &mut service,
                                &request.ticket,
                                Response::Text(content),
                                false,
                                LOAD_LIMITS,
                            )
                            .unwrap();
                        } else {
                            json_schema_load::publish_response(
                                &mut service,
                                &request.ticket,
                                Response::Failed(&units(&format!("Error: Missing {uri}."))),
                                false,
                                LOAD_LIMITS,
                            )
                            .unwrap();
                        }
                    }
                }
            }
        }
        assert!(calls.values().all(|&count| count == 1));
        assert_eq!(
            actual.as_deref(),
            expected.split("|deps:").next(),
            "root={root}, source raw={raw}"
        );
        assert!(matches!(
            Job::start(&mut service, &units(root), workspace, GRAPH_LIMITS).unwrap(),
            Start::Ready(_)
        ));
    }
}

#[test]
fn json_schema_graph는_변경과_취소에서_오래된_공유요청을_거절한다() {
    let mut service = Service::new(&UnusedCompiler, LIMITS);
    let root = units("https://schema.example/root.json");
    let child = units("https://schema.example/b.json");
    let Start::Job(job) = Job::start(&mut service, &root, true, GRAPH_LIMITS).unwrap() else {
        panic!("missing job");
    };
    let Start::Pending(observer) = Job::start(&mut service, &root, true, GRAPH_LIMITS).unwrap()
    else {
        panic!("missing shared observer");
    };
    let Progress::Waiting { job, requests } = job.advance(&mut service).unwrap() else {
        panic!("missing root load");
    };
    assert_eq!(requests.len(), 1);
    json_schema_load::publish_response(
        &mut service,
        &requests[0].ticket,
        Response::Text("{\"$ref\":\"b.json\"}"),
        false,
        LOAD_LIMITS,
    )
    .unwrap();
    let Progress::Waiting { job, requests } = job.advance(&mut service).unwrap() else {
        panic!("missing child load");
    };
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].uri, child);
    assert!(service.on_resource_change(&child).unwrap());
    assert!(matches!(
        job.advance(&mut service),
        Err(EditorError::StaleRevision)
    ));
    assert!(matches!(
        service.read_resolved(&observer),
        Err(EditorError::StaleRevision)
    ));
    assert_eq!(
        json_schema_load::publish_response(
            &mut service,
            &requests[0].ticket,
            Response::Text("{}"),
            false,
            LOAD_LIMITS
        ),
        Err(EditorError::StaleRevision)
    );
    assert!(matches!(
        Job::start(&mut service, &root, true, GRAPH_LIMITS).unwrap(),
        Start::Job(_)
    ));
    let mut replaced = Service::new(&UnusedCompiler, LIMITS);
    replaced.register(&root, None, &[], None, true).unwrap();
    let Start::Job(old) = Job::start(&mut replaced, &root, true, GRAPH_LIMITS).unwrap() else {
        panic!("missing contribution job");
    };
    replaced
        .register(
            &root,
            Some(std::sync::Arc::new(
                taide_native_editor::json_value::parse("{}", VALUE_LIMITS).unwrap(),
            )),
            &[],
            None,
            false,
        )
        .unwrap();
    assert!(matches!(
        old.advance(&mut replaced),
        Err(EditorError::StaleRevision)
    ));
    replaced.clear_external();
    assert!(
        matches!(
            Job::start(&mut replaced, &root, true, GRAPH_LIMITS).unwrap(),
            Start::Job(_)
        ),
        "retired unissued request survived restoration"
    );
}

#[test]
fn json_schema_graph는_문서_요청_자료_작업예산_실패에서_부분cache를_게시하지않는다() {
    let root = units("https://schema.example/root.json");
    let mut service = Service::new(&UnusedCompiler, LIMITS);
    assert!(matches!(
        Job::start(
            &mut service,
            &root,
            true,
            json_schema_graph::Limits {
                max_requests: 0,
                ..GRAPH_LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        service.begin_load(&root).unwrap(),
        LoadState::Start(_)
    ));
    for limits in [
        json_schema_graph::Limits {
            max_documents: 1,
            ..GRAPH_LIMITS
        },
        json_schema_graph::Limits {
            max_uri_units: root.len(),
            ..GRAPH_LIMITS
        },
        json_schema_graph::Limits {
            refs: taide_native_editor::json_schema_refs::Limits {
                max_work: 0,
                ..REF_LIMITS
            },
            ..GRAPH_LIMITS
        },
    ] {
        let mut service = Service::new(&UnusedCompiler, LIMITS);
        let source = std::sync::Arc::new(
            taide_native_editor::json_value::parse("{\"$ref\":\"b.json\"}", VALUE_LIMITS).unwrap(),
        );
        service
            .register(&root, Some(source), &[], None, false)
            .unwrap();
        let Start::Job(job) = Job::start(&mut service, &root, true, limits).unwrap() else {
            panic!("missing limited job");
        };
        assert!(matches!(
            job.advance(&mut service),
            Err(EditorError::Capacity)
        ));
        assert!(matches!(
            service.begin_resolve(&root).unwrap(),
            ResolveState::Start {
                source: LoadState::Ready(_),
                ..
            }
        ));
    }
}

#[test]
fn json_schema_graph는_가져온전체값_문자와_오류메시지예산을_지킨다() {
    let root = units("https://schema.example/root.json");
    for (limits, content) in [
        (
            json_schema_graph::Limits {
                refs: taide_native_editor::json_schema_refs::Limits {
                    values: taide_native_editor::json_value::Limits {
                        max_values: 2,
                        ..VALUE_LIMITS
                    },
                    ..REF_LIMITS
                },
                ..GRAPH_LIMITS
            },
            "{}",
        ),
        (
            json_schema_graph::Limits {
                refs: taide_native_editor::json_schema_refs::Limits {
                    values: taide_native_editor::json_value::Limits {
                        max_text_units: 10,
                        ..VALUE_LIMITS
                    },
                    ..REF_LIMITS
                },
                ..GRAPH_LIMITS
            },
            "{\"title\":\"x\"}",
        ),
        (
            json_schema_graph::Limits {
                refs: taide_native_editor::json_schema_refs::Limits {
                    max_messages: 0,
                    ..REF_LIMITS
                },
                ..GRAPH_LIMITS
            },
            "\u{feff} {}",
        ),
    ] {
        let mut service = Service::new(&UnusedCompiler, LIMITS);
        service
            .register(
                &root,
                Some(std::sync::Arc::new(
                    taide_native_editor::json_value::parse("{\"$ref\":\"b.json\"}", VALUE_LIMITS)
                        .unwrap(),
                )),
                &[],
                None,
                false,
            )
            .unwrap();
        let Start::Job(job) = Job::start(&mut service, &root, true, limits).unwrap() else {
            panic!("missing limited graph");
        };
        let Progress::Waiting { job, requests } = job.advance(&mut service).unwrap() else {
            panic!("missing external request");
        };
        assert_eq!(requests.len(), 1);
        json_schema_load::publish_response(
            &mut service,
            &requests[0].ticket,
            Response::Text(content),
            false,
            LOAD_LIMITS,
        )
        .unwrap();
        assert!(matches!(
            job.advance(&mut service),
            Err(EditorError::Capacity)
        ));
        assert!(matches!(
            service.begin_resolve(&root).unwrap(),
            ResolveState::Start {
                source: LoadState::Ready(_),
                ..
            }
        ));
    }
}

#[test]
fn json_schema_graph는_실제전이의존과_원자적최종게시를_지킨다() {
    let root = units("https://schema.example/root.json");
    let child = units("https://schema.example/b.json");
    let leaf = units("https://schema.example/c.json");
    for service_limits in [
        LIMITS,
        taide_native_editor::json_schema_service::Limits {
            max_values: 4,
            ..LIMITS
        },
    ] {
        let mut service = Service::new(&UnusedCompiler, service_limits);
        service
            .register(
                &root,
                Some(std::sync::Arc::new(
                    taide_native_editor::json_value::parse("{\"$ref\":\"b.json\"}", VALUE_LIMITS)
                        .unwrap(),
                )),
                &[],
                None,
                false,
            )
            .unwrap();
        let Start::Job(mut job) = Job::start(&mut service, &root, true, GRAPH_LIMITS).unwrap()
        else {
            panic!("missing graph");
        };
        let mut ready = false;
        let is_limited = service_limits.max_values < LIMITS.max_values;
        loop {
            match job.advance(&mut service) {
                Ok(Progress::Ready(_)) => {
                    ready = true;
                    break;
                }
                Ok(Progress::Waiting {
                    job: next,
                    requests,
                }) => {
                    job = next;
                    for request in requests {
                        let body = if request.uri == child && !is_limited {
                            "{\"$ref\":\"c.json\"}"
                        } else {
                            "{\"type\":\"string\"}"
                        };
                        json_schema_load::publish_response(
                            &mut service,
                            &request.ticket,
                            Response::Text(body),
                            false,
                            LOAD_LIMITS,
                        )
                        .unwrap();
                    }
                }
                Err(EditorError::Capacity) if is_limited => break,
                _ => panic!("unexpected graph state"),
            }
        }
        assert_eq!(ready, !is_limited);
        if is_limited {
            assert!(service.on_resource_change(&child).unwrap());
            assert!(
                matches!(service.begin_load(&root).unwrap(), LoadState::Ready(_)),
                "failed graph partially published dependencies"
            );
        } else {
            assert!(service.on_resource_change(&leaf).unwrap());
            assert!(matches!(
                service.begin_load(&child).unwrap(),
                LoadState::Start(_)
            ));
            assert!(matches!(
                service.begin_load(&root).unwrap(),
                LoadState::Start(_)
            ));
        }
    }
}
