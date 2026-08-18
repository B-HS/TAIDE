use std::path::{Path, PathBuf};

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use tauri::{AppHandle, Manager};
use tokio::io::AsyncReadExt;

use crate::infra::range_file::{extension_mime, parse_range, read_slice, RANGE_RESPONSE_CSP};
use crate::infra::root_guard;
use crate::state::AppState;

const BROWSER_CSP: &str = "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; frame-src 'self' blob:; media-src 'self' blob:; font-src 'self' data:; worker-src 'self' blob:; script-src 'self'; connect-src 'self' ws: wss:";
const DEV_SERVER_URL: &str = "http://localhost:5173";
/// Chunk size for the full-file (non-`Range`) streaming fallback below — bounds
/// [`stream_file_body`]'s per-read memory footprint independent of the served file's total size.
const FULL_FILE_STREAM_CHUNK_BYTES: usize = 64 * 1024;
const INDEX_DOCUMENT: &str = "index.html";

fn no_store() -> HeaderValue {
    HeaderValue::from_static("no-store")
}

async fn proxy_dev(uri: &Uri) -> Response {
    let path_and_query = uri.path_and_query().map(|value| value.as_str()).unwrap_or("/");
    let target = format!("{DEV_SERVER_URL}{path_and_query}");

    let response = match reqwest::Client::new().get(&target).send().await {
        Ok(response) => response,
        Err(_) => return (StatusCode::BAD_GATEWAY, "dev 서버에 연결할 수 없습니다").into_response(),
    };

    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => return (StatusCode::BAD_GATEWAY, "dev 응답을 읽을 수 없습니다").into_response(),
    };

    let mut builder = Response::builder().status(status.as_u16());
    if let Some(content_type) = content_type {
        if let Ok(value) = HeaderValue::from_str(&content_type) {
            builder = builder.header(header::CONTENT_TYPE, value);
        }
    }
    builder = builder.header(header::CACHE_CONTROL, no_store());
    builder
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

fn serve_embedded(app: &AppHandle, path: &str) -> Response {
    let resolver = app.asset_resolver();
    let asset = resolver.get(path.to_string()).or_else(|| resolver.get(INDEX_DOCUMENT.to_string()));

    let Some(asset) = asset else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut builder = Response::builder().status(StatusCode::OK);
    if let Ok(mime) = HeaderValue::from_str(&asset.mime_type) {
        builder = builder.header(header::CONTENT_TYPE, mime);
    }
    builder = builder.header(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(BROWSER_CSP));
    builder = builder.header(header::CACHE_CONTROL, no_store());
    builder
        .body(Body::from(asset.bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub async fn serve_static(State(app): State<AppHandle>, uri: Uri) -> Response {
    if tauri::is_dev() {
        return proxy_dev(&uri).await;
    }

    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { INDEX_DOCUMENT } else { path };
    serve_embedded(&app, path)
}

fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    query?.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        if name != key {
            return None;
        }
        Some(percent_decode(value))
    })
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hi = (bytes[index + 1] as char).to_digit(16);
                let lo = (bytes[index + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    decoded.push((hi * 16 + lo) as u8);
                    index += 3;
                    continue;
                }
                decoded.push(bytes[index]);
                index += 1;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

pub async fn file_range(State(app): State<AppHandle>, uri: Uri, headers: HeaderMap) -> Response {
    let Some(requested) = query_param(uri.query(), "path") else {
        return (StatusCode::BAD_REQUEST, "path 쿼리가 필요합니다").into_response();
    };

    let resolved = {
        let state = app.state::<AppState>();
        let projects = state.projects.read();
        root_guard::resolve_owning_project(&projects, Path::new(&requested))
    };
    let Ok((_, resolved)) = resolved else {
        return StatusCode::FORBIDDEN.into_response();
    };

    let Ok(metadata) = std::fs::metadata(&resolved) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let total = metadata.len();
    let mime = extension_mime(&resolved);

    let range_header = headers.get(header::RANGE).and_then(|value| value.to_str().ok()).map(str::to_string);

    if let Some(range_header) = range_header {
        let Some((start, end)) = parse_range(&range_header, total) else {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
                .body(Body::empty())
                .unwrap_or_else(|_| StatusCode::RANGE_NOT_SATISFIABLE.into_response());
        };

        let length = end - start + 1;
        let Ok(bytes) = read_slice(&resolved, start, length) else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        return Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, HeaderValue::from_static(mime))
            .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{total}"))
            .header(header::CONTENT_LENGTH, length)
            .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
            .header(header::CACHE_CONTROL, no_store())
            .header(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(RANGE_RESPONSE_CSP))
            .header(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"))
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    let Ok(body) = stream_file_body(resolved).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static(mime))
        .header(header::CONTENT_LENGTH, total)
        .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
        .header(header::CACHE_CONTROL, no_store())
        .header(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(RANGE_RESPONSE_CSP))
        .header(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"))
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Streams `path`'s full contents as an axum [`Body`] in [`FULL_FILE_STREAM_CHUNK_BYTES`]-sized
/// chunks instead of buffering the whole file into memory up front — the fallback [`file_range`]
/// takes whenever a client requests without a `Range` header (browsers normally only omit `Range`
/// for smaller assets, but nothing stops a client from fetching a multi-gigabyte project file this
/// way). `Range` requests never had this problem: `parse_range` already caps a single request's
/// read at `RANGE_CHUNK_LIMIT` via `read_slice`, above.
async fn stream_file_body(path: PathBuf) -> std::io::Result<Body> {
    use futures_util::stream;

    let file = tokio::fs::File::open(&path).await?;
    let chunks = stream::unfold((file, false), |(mut file, done)| async move {
        if done {
            return None;
        }
        let mut buffer = vec![0u8; FULL_FILE_STREAM_CHUNK_BYTES];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(read_bytes) => {
                buffer.truncate(read_bytes);
                Some((Ok::<_, std::io::Error>(Bytes::from(buffer)), (file, false)))
            }
            Err(error) => Some((Err(error), (file, true))),
        }
    });
    Ok(Body::from_stream(chunks))
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;

    use super::*;

    fn temp_file(name: &str, content: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("taide-serving-test-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, content).unwrap();
        path
    }

    async fn collect_body(body: Body) -> Vec<u8> {
        let mut collected = Vec::new();
        let mut stream = body.into_data_stream();
        while let Some(chunk) = stream.next().await {
            collected.extend_from_slice(&chunk.unwrap());
        }
        collected
    }

    #[tokio::test]
    async fn 파일_스트리밍은_한_청크보다_작은_파일도_그대로_재구성한다() {
        let content = b"hello, taide".to_vec();
        let path = temp_file("small", &content);

        let body = stream_file_body(path.clone()).await.unwrap();
        assert_eq!(collect_body(body).await, content);

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn 파일_스트리밍은_여러_청크에_걸친_파일도_바이트_그대로_재구성한다() {
        let content: Vec<u8> = (0..(FULL_FILE_STREAM_CHUNK_BYTES * 3 + 123))
            .map(|index| (index % 251) as u8)
            .collect();
        let path = temp_file("large", &content);

        let body = stream_file_body(path.clone()).await.unwrap();
        assert_eq!(collect_body(body).await, content);

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn 파일_스트리밍은_존재하지_않는_경로에서_에러를_반환한다() {
        let missing = std::env::temp_dir().join(format!("taide-serving-test-missing-{}", uuid::Uuid::new_v4()));
        assert!(stream_file_body(missing).await.is_err());
    }
}
