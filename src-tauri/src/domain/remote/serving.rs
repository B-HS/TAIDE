use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use tauri::{AppHandle, Manager};

use crate::infra::root_guard;
use crate::state::AppState;

const BROWSER_CSP: &str = "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; frame-src 'self' blob:; media-src 'self' blob:; font-src 'self' data:; worker-src 'self' blob:; script-src 'self'; connect-src 'self' ws: wss:";
const DEV_SERVER_URL: &str = "http://localhost:5173";
const RANGE_CHUNK_LIMIT: u64 = 1000 * 1024;
const INDEX_DOCUMENT: &str = "index.html";

fn no_store() -> HeaderValue {
    HeaderValue::from_static("no-store")
}

fn extension_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("mkv") => "video/x-matroska",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
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

fn parse_range(range_header: &str, total: u64) -> Option<(u64, u64)> {
    let spec = range_header.strip_prefix("bytes=")?;
    let (start_raw, end_raw) = spec.split_once('-')?;

    let start = if start_raw.is_empty() { 0 } else { start_raw.parse::<u64>().ok()? };
    if start >= total {
        return None;
    }

    let requested_end = if end_raw.is_empty() {
        total - 1
    } else {
        end_raw.parse::<u64>().ok()?
    };
    let capped_end = requested_end.min(total - 1).min(start + RANGE_CHUNK_LIMIT - 1);
    Some((start, capped_end))
}

fn read_slice(path: &Path, start: u64, length: u64) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = vec![0u8; length as usize];
    file.read_exact(&mut buffer)?;
    Ok(buffer)
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
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    let Ok(bytes) = std::fs::read(&resolved) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static(mime))
        .header(header::CONTENT_LENGTH, total)
        .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
        .header(header::CACHE_CONTROL, no_store())
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
