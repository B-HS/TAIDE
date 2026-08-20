//! Serves file bytes to the webview under the `asset://` scheme (`http://asset.localhost` on
//! Windows/Android) — a from-scratch replacement for Tauri's built-in handler
//! (`tauri::protocol::asset`, gated by the `protocol-asset` Cargo feature) so that "is this path
//! readable" is decided by the *live* set of currently open projects (`AppState::projects`)
//! instead of Tauri's `asset_protocol_scope()` (`scope::fs::Scope`), whose `allow_directory` and
//! `forbid_directory` are both append-only with no way to undo either — see `docs/architecture.md`
//! §6.3 for why that made the scope impossible to revoke when a project closes.
//!
//! Registering our own handler under the same scheme name ("asset") makes Tauri skip its built-in
//! registration entirely — confirmed by reading the vendored source at
//! `tauri-2.11.5/src/manager/webview.rs`'s `prepare_pending_webview`, which only wires up the
//! built-in asset protocol `if !registered_scheme_protocols.contains(&"asset".into())`.
//! `convertFileSrc` (`@tauri-apps/api/core`) and the `asset:`/`http://asset.localhost` CSP
//! sources in `tauri.conf.json` need no change: both depend only on the scheme *name*, not on
//! which side answers requests for it.
//!
//! [`respond`] mirrors the shape of `domain::remote::serving::file_range` (single-range,
//! `Range` support for `<video>`/`<audio>` scrubbing, via the primitives shared in
//! [`crate::infra::range_file`]) and, for the parts the built-in handler could not be reused for,
//! the vendored `tauri::protocol::asset` module itself (unreachable from application code — its
//! parent `protocol` module is declared `pub(crate)` in `tauri-2.11.5/src/lib.rs`, and
//! `get_response` is a private `fn` even within that module). `is_allowed` there is a
//! `scope.is_allowed(&path)` lookup against the append-only scope; here it is
//! [`root_guard::resolve_owning_project`] against the live open-project set — the same function
//! that already gates the remote `/__taide/file` route, so a path escapes this handler only if it
//! would also escape that one (nested-project tie-breaking, symlink canonicalization, everything
//! `root_guard`'s own test suite already covers).
//!
//! [`respond`] itself takes the open-project map as a plain parameter rather than reaching into
//! `AppState` — matching every other `root_guard`-gated function's shape — so the app-handle/state
//! lookup lives at the one call site that actually needs an `AppHandle` (`lib.rs`'s
//! `register_uri_scheme_protocol("asset", ...)` registration, which wraps this function in a
//! closure). This keeps the handler itself unit-testable without a live `AppHandle` (see the tests
//! module below) and keeps `infra::` free of a direct `crate::state` dependency, same as
//! `infra::root_guard`.
//!
//! Response headers omit the built-in handler's `Access-Control-Allow-Origin`/
//! `Access-Control-Expose-Headers: content-range` — this handler's only current consumers
//! (`<video>`/`<audio src={convertFileSrc(...)}>` in `preview-pane.tsx`) load it as a same-origin
//! element `src`, which never triggers a CORS preflight or reads response headers via `fetch`/XHR,
//! so the omission has no observed effect. Documented here (rather than silently diverging) because
//! a future consumer that *does* `fetch()` an `asset://` URL directly would need those headers
//! reinstated; `UriSchemeContext` does not expose the webview's origin the way the built-in
//! handler's `window_origin` parameter did, so adding them back requires either threading that
//! value through the registration closure or resolving it via `context.webview_label()` at the
//! call site.
//!
//! KNOWN ISSUE (`docs/acknowledge/2026-08-18-audit-t1-batch2-contract.md` Phase R2): this handler
//! could not be exercised against a live webview — no `<video>`/`<audio>` scrubbing regression
//! check, no confirmation that WKWebView/WebView2 actually route `asset://` requests through a
//! `register_uri_scheme_protocol`-registered handler the same way they route the built-in one.
//! See the qa6 checklist (owed to the Phase D documentation pass per the batch contract) for the
//! outstanding manual verification item.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tauri::http::{header, HeaderValue, Request, Response, StatusCode};

use crate::domain::project::types::Project;
use crate::ids::ProjectId;
use crate::infra::range_file::{extension_mime, parse_range, read_slice, RANGE_RESPONSE_CSP};
use crate::infra::root_guard;

fn no_store() -> HeaderValue {
    HeaderValue::from_static("no-store")
}

/// Decodes the request path Tauri hands the scheme handler back into the absolute filesystem path
/// that `convertFileSrc` encoded with `encodeURIComponent` (`tauri-2.11.5/scripts/core.js`):
/// `${protocol}://localhost/${encodeURIComponent(filePath)}` — the *first* `/` is the URL's own
/// path separator (stripped here), everything after it is the fully percent-encoded original
/// absolute path (whose own leading `/` therefore arrives as a literal `%2F`, decoded back below).
///
/// Deliberately does **not** decode `+` as a space the way a query-string decoder would
/// (`domain::remote::serving::percent_decode`, used there for actual query parameters) — that's
/// an `application/x-www-form-urlencoded` convention that doesn't apply to a URL path segment, and
/// `encodeURIComponent` never emits a raw `+` for one anyway (it escapes `+` itself to `%2B`).
fn decode_asset_path(request_path: &str) -> Option<String> {
    let encoded = request_path.strip_prefix('/')?;
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hi = (bytes[index + 1] as char).to_digit(16);
            let lo = (bytes[index + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                decoded.push((hi * 16 + lo) as u8);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    Some(String::from_utf8_lossy(&decoded).into_owned())
}

/// Builds an empty-body response for `status` without ever going through a fallible
/// `Response::builder()` — no headers are set, so there is nothing for that builder to reject and
/// no `Result` to `unwrap`/`expect` away.
fn status_only(status: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = status;
    response
}

/// Resolves `requested` against the *currently open* project set (the same map
/// `project_open`/`project_close` maintain in `AppState::projects`), returning the canonicalized
/// on-disk path only if it falls within one of them. This is the entire revocation story: closing a
/// project removes it from that map, so a request for a path under its root fails here on the very
/// next request — no separate "forbid" step, no stale scope entries to clean up.
fn resolve_within_open_project(projects: &HashMap<ProjectId, Project>, requested: &str) -> Option<PathBuf> {
    root_guard::resolve_owning_project(projects, Path::new(requested))
        .ok()
        .map(|(_, resolved)| resolved)
}

/// The body of the `register_uri_scheme_protocol("asset", ...)` handler — see the module doc for
/// what this replaces and why. `projects` is the live open-project snapshot the `lib.rs`
/// registration closure reads from `AppState` immediately before calling this function.
pub fn respond(projects: &HashMap<ProjectId, Project>, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some(requested) = decode_asset_path(request.uri().path()) else {
        return status_only(StatusCode::FORBIDDEN);
    };

    let Some(resolved) = resolve_within_open_project(projects, &requested) else {
        return status_only(StatusCode::FORBIDDEN);
    };

    let Ok(metadata) = std::fs::metadata(&resolved) else {
        return status_only(StatusCode::NOT_FOUND);
    };
    let total = metadata.len();
    let mime = extension_mime(&resolved);

    let range_header = request.headers().get(header::RANGE).and_then(|value| value.to_str().ok());

    if let Some(range_header) = range_header {
        let Some((start, end)) = parse_range(range_header, total) else {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
                .body(Vec::new())
                .unwrap_or_else(|_| status_only(StatusCode::RANGE_NOT_SATISFIABLE));
        };

        let length = end - start + 1;
        let Ok(bytes) = read_slice(&resolved, start, length) else {
            return status_only(StatusCode::INTERNAL_SERVER_ERROR);
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
            .body(bytes)
            .unwrap_or_else(|_| status_only(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let Ok(bytes) = std::fs::read(&resolved) else {
        return status_only(StatusCode::NOT_FOUND);
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static(mime))
        .header(header::CONTENT_LENGTH, total)
        .header(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"))
        .header(header::CACHE_CONTROL, no_store())
        .header(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(RANGE_RESPONSE_CSP))
        .header(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"))
        .body(bytes)
        .unwrap_or_else(|_| status_only(StatusCode::INTERNAL_SERVER_ERROR))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_project(dir: &Path) -> HashMap<ProjectId, Project> {
        let mut projects = HashMap::new();
        projects.insert(
            ProjectId::from("project-1".to_string()),
            Project {
                id: ProjectId::from("project-1".to_string()),
                root: dir.to_string_lossy().to_string(),
                name: "project".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
            },
        );
        projects
    }

    fn asset_request(file_path: &Path, range: Option<&str>) -> Request<Vec<u8>> {
        let uri = format!("asset://localhost/{}", url_encode(&file_path.to_string_lossy()));
        let mut builder = Request::builder().uri(uri);
        if let Some(range) = range {
            builder = builder.header(header::RANGE, range);
        }
        builder.body(Vec::new()).unwrap()
    }

    #[test]
    fn 요청_경로는_선행_슬래시를_벗기고_퍼센트_인코딩을_복원한다() {
        let encoded = format!("/{}", url_encode("/Users/dev/my project/video.mp4"));
        assert_eq!(decode_asset_path(&encoded).as_deref(), Some("/Users/dev/my project/video.mp4"));
    }

    #[test]
    fn 플러스_기호는_공백이_아니라_그대로_보존된다() {
        let encoded = format!("/{}", url_encode("/Users/dev/C++/notes.txt"));
        assert_eq!(decode_asset_path(&encoded).as_deref(), Some("/Users/dev/C++/notes.txt"));
    }

    #[test]
    fn 선행_슬래시가_없는_요청_경로는_거부된다() {
        assert_eq!(decode_asset_path("no-leading-slash"), None);
    }

    #[test]
    fn 열린_프로젝트_루트_하위_경로만_허용되고_닫힌_프로젝트는_즉시_거부된다() {
        let dir = std::env::temp_dir().join(format!("taide-asset-protocol-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("clip.mp4");
        std::fs::write(&file_path, b"fake video bytes").unwrap();

        let open_projects = open_project(&dir);
        assert!(root_guard::resolve_owning_project(&open_projects, &file_path).is_ok());

        let closed_projects = HashMap::new();
        assert!(
            root_guard::resolve_owning_project(&closed_projects, &file_path).is_err(),
            "프로젝트가 닫혀 open_projects 에서 제거되면 같은 경로가 즉시 거부되어야 한다"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 열린_프로젝트의_파일은_범위_없는_요청에_전체_바이트를_반환한다() {
        let dir = std::env::temp_dir().join(format!("taide-asset-protocol-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("notes.txt");
        std::fs::write(&file_path, b"hello asset protocol").unwrap();

        let projects = open_project(&dir);
        let response = respond(&projects, asset_request(&file_path, None));

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get(header::CACHE_CONTROL).unwrap(), "no-store");
        assert_eq!(response.body(), b"hello asset protocol");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 닫힌_프로젝트의_파일_요청은_403을_반환한다() {
        let dir = std::env::temp_dir().join(format!("taide-asset-protocol-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("notes.txt");
        std::fs::write(&file_path, b"hello").unwrap();

        let empty_projects = HashMap::new();
        let response = respond(&empty_projects, asset_request(&file_path, None));

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 범위_요청은_206과_함께_요청된_슬라이스만_반환한다() {
        let dir = std::env::temp_dir().join(format!("taide-asset-protocol-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("clip.mp4");
        std::fs::write(&file_path, b"0123456789").unwrap();

        let projects = open_project(&dir);
        let response = respond(&projects, asset_request(&file_path, Some("bytes=2-4")));

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.body(), b"234");
        assert_eq!(response.headers().get(header::CONTENT_RANGE).unwrap(), "bytes 2-4/10");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 뒤집힌_범위_요청은_패닉_없이_416을_반환한다() {
        let dir = std::env::temp_dir().join(format!("taide-asset-protocol-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("clip.mp4");
        std::fs::write(&file_path, b"0123456789").unwrap();

        let projects = open_project(&dir);
        let response = respond(&projects, asset_request(&file_path, Some("bytes=8-2")));

        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);

        std::fs::remove_dir_all(&dir).ok();
    }

    fn url_encode(value: &str) -> String {
        value
            .bytes()
            .map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => {
                    (byte as char).to_string()
                }
                _ => format!("%{byte:02X}"),
            })
            .collect()
    }
}
