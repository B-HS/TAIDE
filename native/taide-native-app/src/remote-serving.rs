use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Path as ExtractedPath, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use futures_util::stream;
use taide_infra::range_file::{RANGE_RESPONSE_CSP, extension_mime, parse_range, read_slice};
use taide_infra::root_guard;
use tokio::io::AsyncReadExt;

use crate::remote_http::Context;

const STREAM_CHUNK_BYTES: usize = 64 * 1024;
const BROWSER_CSP: &str = "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; frame-src 'self' blob:; media-src 'self' blob:; font-src 'self' data:; worker-src 'self' blob:; script-src 'self'; connect-src 'self' ws: wss:";
const INDEX_DOCUMENT: &str = "index.html";
const HEX_RADIX: u32 = 16;
const HEX_DIGITS: u32 = 16;
const PERCENT_SEQUENCE_BYTES: usize = 3;

pub struct Asset {
    pub mime: String,
    pub bytes: Vec<u8>,
}

pub type AssetResolver = Arc<dyn Fn(&str) -> Option<Asset> + Send + Sync>;

pub(crate) async fn serve_static(State(context): State<Context>, uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() {
        INDEX_DOCUMENT
    } else {
        path
    };
    let asset = (context.ports.assets)(path).or_else(|| (context.ports.assets)(INDEX_DOCUMENT));
    let Some(asset) = asset else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut response = Response::new(Body::from(asset.bytes));
    if let Ok(mime) = HeaderValue::from_str(&asset.mime) {
        response.headers_mut().insert(header::CONTENT_TYPE, mime);
    }
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(BROWSER_CSP),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(crate) async fn file_range(
    State(context): State<Context>,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let requested = uri.query().and_then(|query| {
        query.split('&').find_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            (name == "path").then(|| percent_decode(value))
        })
    });
    let Some(requested) = requested else {
        return (StatusCode::BAD_REQUEST, "path 쿼리가 필요합니다").into_response();
    };
    serve_project_file(context, requested, headers).await
}

pub(crate) async fn file_path(
    State(context): State<Context>,
    ExtractedPath(path): ExtractedPath<String>,
    headers: HeaderMap,
) -> Response {
    serve_project_file(context, path, headers).await
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + PERCENT_SEQUENCE_BYTES <= bytes.len()
            && let (Some(high), Some(low)) = (
                char::from(bytes[index + 1]).to_digit(HEX_RADIX),
                char::from(bytes[index + PERCENT_SEQUENCE_BYTES - 1]).to_digit(HEX_RADIX),
            )
            && let Ok(byte) = u8::try_from(high * HEX_DIGITS + low)
        {
            decoded.push(byte);
            index += PERCENT_SEQUENCE_BYTES;
            continue;
        }
        decoded.push(if bytes[index] == b'+' {
            b' '
        } else {
            bytes[index]
        });
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

async fn serve_project_file(context: Context, requested: String, headers: HeaderMap) -> Response {
    let resolved = root_guard::resolve_owning_project(
        &context.services.state.projects.read(),
        Path::new(&requested),
    );
    let Ok((_, path)) = resolved else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let Ok(metadata) = std::fs::metadata(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let total = metadata.len();
    let mime = extension_mime(&path);
    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    if let Some(range) = range {
        let Some((start, end)) = parse_range(range, total) else {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
                .body(Body::empty())
                .unwrap_or_else(|_| StatusCode::RANGE_NOT_SATISFIABLE.into_response());
        };
        let length = end - start + 1;
        let Ok(bytes) = read_slice(&path, start, length) else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        return Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, HeaderValue::from_static(mime))
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{total}"),
            )
            .header(header::CONTENT_LENGTH, length)
            .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
            .header(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))
            .header(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(RANGE_RESPONSE_CSP),
            )
            .header(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            )
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }
    let Ok(body) = stream_file_body(path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static(mime))
        .header(header::CONTENT_LENGTH, total)
        .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))
        .header(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(RANGE_RESPONSE_CSP),
        )
        .header(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        )
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn stream_file_body(path: PathBuf) -> std::io::Result<Body> {
    let file = tokio::fs::File::open(path).await?;
    let chunks = stream::unfold((file, false), |(mut file, done)| async move {
        if done {
            return None;
        }
        let mut bytes = vec![0u8; STREAM_CHUNK_BYTES];
        match file.read(&mut bytes).await {
            Ok(0) => None,
            Ok(count) => {
                bytes.truncate(count);
                Some((Ok::<_, std::io::Error>(Bytes::from(bytes)), (file, false)))
            }
            Err(error) => Some((Err(error), (file, true))),
        }
    });
    Ok(Body::from_stream(chunks))
}
