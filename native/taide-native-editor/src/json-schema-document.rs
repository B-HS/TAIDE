use std::sync::Arc;

use crate::diagnostics::Marker;
use crate::document::EditorError;
use crate::json_parser;
use crate::json_schema_associations::PatternCompiler as AssociationCompiler;
use crate::json_schema_graph::{self, Job, Progress, Start};
use crate::json_schema_load::{self, Response};
use crate::json_schema_policy::{self, Policy};
use crate::json_schema_refs::Resolved;
use crate::json_schema_service::{self, Service};
use crate::json_schema_string::PatternCompiler;
use crate::json_validation::Settings;
use crate::json_value::{self, Arena};

const CONTRIBUTIONS: [(&str, &str); 2] = [
    (
        "http://json-schema.org/draft-04/schema#",
        include_str!("../resources/json-schema-draft-04.json"),
    ),
    (
        "http://json-schema.org/draft-07/schema#",
        include_str!("../resources/json-schema-draft-07.json"),
    ),
];

/// A schema bundled with the language service (`schemaContributions`).
#[derive(Clone, Debug)]
pub struct Contribution {
    pub uri: Vec<u16>,
    pub schema: Arc<Arena>,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub service: json_schema_service::Limits,
    pub graph: json_schema_graph::Limits,
    pub load: json_schema_load::Limits,
    pub policy: json_schema_policy::Limits,
    pub contributions: json_value::Limits,
    pub max_steps: usize,
}

/// Parses the bundled draft-04/draft-07 meta schemas in the original registration order.
pub fn contributions(limits: json_value::Limits) -> Result<Vec<Contribution>, EditorError> {
    CONTRIBUTIONS
        .iter()
        .map(|(uri, source)| {
            Ok(Contribution {
                uri: uri.encode_utf16().collect(),
                schema: Arc::new(json_value::parse(source, limits)?),
            })
        })
        .collect()
}

/// Runs the language service `doValidation` for one document without a schema request service:
/// bundled contributions resolve locally and every other schema load reports the original
/// "no schema request service" error.
#[allow(clippy::too_many_arguments)]
pub fn diagnostics(
    input: &str,
    resource: &str,
    language: &str,
    settings: Settings,
    policy: Policy,
    contributions: &[Contribution],
    associations: &dyn AssociationCompiler,
    patterns: &dyn PatternCompiler,
    limits: Limits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<Marker>, EditorError> {
    if !settings.validate {
        return Ok(Vec::new());
    }
    let document = json_parser::parse(input, true, limits.policy.syntax.parser)?;
    let mut service = Service::new(associations, limits.service);
    for contribution in contributions {
        service.register(
            &contribution.uri,
            Some(contribution.schema.clone()),
            &[],
            None,
            true,
        )?;
    }
    let resource = resource.encode_utf16().collect::<Vec<_>>();
    let resolved = match service.select(&resource, Some(&document))? {
        None => None,
        Some(id) => Some(resolve(&mut service, &id, limits, cancelled)?),
    };
    if cancelled() {
        return Err(EditorError::StaleRevision);
    }
    json_schema_policy::document_diagnostics(
        &document,
        language,
        settings,
        policy,
        resolved.as_deref(),
        patterns,
        limits.policy,
    )
}

fn resolve(
    service: &mut Service<'_>,
    id: &[u16],
    limits: Limits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Arc<Resolved>, EditorError> {
    let mut job = match Job::start(service, id, true, limits.graph)? {
        Start::Ready(resolved) => return Ok(resolved),
        Start::Pending(_) => return Err(EditorError::InvalidBoundary),
        Start::Job(job) => job,
    };
    for _ in 0..limits.max_steps {
        if cancelled() {
            job.abort(service);
            return Err(EditorError::StaleRevision);
        }
        match job.advance(service)? {
            Progress::Ready(resolved) => return Ok(resolved),
            Progress::Waiting {
                job: next,
                requests,
            } => {
                job = next;
                if requests.is_empty() {
                    job.abort(service);
                    return Err(EditorError::InvalidBoundary);
                }
                for request in requests {
                    json_schema_load::publish_response(
                        service,
                        &request.ticket,
                        Response::Unavailable,
                        cfg!(windows),
                        limits.load,
                    )?;
                }
            }
        }
    }
    job.abort(service);
    Err(EditorError::Capacity)
}
