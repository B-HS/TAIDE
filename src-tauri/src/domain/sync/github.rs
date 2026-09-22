use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::sync::types::{SYNC_GIST_DESCRIPTION, SYNC_GIST_FILENAME};
use crate::error::{AppError, AppResult};
use crate::infra::redact::mask_provider_error;

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const USER_AGENT: &str = "TAIDE-Sync";
const GIST_PAGE_SIZE: usize = 100;

#[derive(Debug, Serialize)]
struct GistFileWrite<'a> {
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct CreateOrUpdateGistBody<'a> {
    description: &'a str,
    public: bool,
    files: HashMap<&'a str, GistFileWrite<'a>>,
}

fn build_gist_body(payload_json: &str) -> CreateOrUpdateGistBody<'_> {
    let mut files = HashMap::new();
    files.insert(SYNC_GIST_FILENAME, GistFileWrite { content: payload_json });
    CreateOrUpdateGistBody {
        description: SYNC_GIST_DESCRIPTION,
        public: false,
        files,
    }
}

#[derive(Debug, Default, Deserialize)]
struct GistFileRead {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GistResponse {
    id: String,
    updated_at: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    files: HashMap<String, GistFileRead>,
}

pub struct GistClient<'a> {
    pub client: &'a reqwest::Client,
    pub token: &'a str,
}

impl GistClient<'_> {
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    fn request(&self, method: reqwest::Method, url: String) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .header("authorization", self.auth_header())
            .header("user-agent", USER_AGENT)
            .header("accept", "application/vnd.github+json")
            .header("x-github-api-version", GITHUB_API_VERSION)
    }

    async fn parse_gist_response(res: reqwest::Response) -> AppResult<GistResponse> {
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "GitHub gist request failed ({status}): {}",
                mask_provider_error(&body)
            )));
        }
        res.json::<GistResponse>()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))
    }

    pub async fn discover_sync_gist(&self, preferred_id: Option<&str>) -> AppResult<Option<(String, String)>> {
        self.discover_sync_gist_at(GITHUB_API_BASE, preferred_id).await
    }

    async fn discover_sync_gist_at(&self, api_base: &str, preferred_id: Option<&str>) -> AppResult<Option<(String, String)>> {
        let mut page = 1;
        let mut newest: Option<(String, String)> = None;
        loop {
            let response = self
                .request(reqwest::Method::GET, format!("{api_base}/gists"))
                .query(&[("per_page", GIST_PAGE_SIZE), ("page", page)])
                .send()
                .await
                .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;
            if !response.status().is_success() {
                return Err(AppError::InvalidArgument(
                    "GitHub personal access token was rejected or gist discovery failed".to_string(),
                ));
            }
            let gists = response
                .json::<Vec<GistResponse>>()
                .await
                .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;
            let is_last_page = gists.len() < GIST_PAGE_SIZE;
            for gist in gists {
                if gist.description.as_deref() != Some(SYNC_GIST_DESCRIPTION) || !gist.files.contains_key(SYNC_GIST_FILENAME) {
                    continue;
                }
                if Some(gist.id.as_str()) == preferred_id {
                    return Ok(Some((gist.id, gist.updated_at)));
                }
                if newest
                    .as_ref()
                    .is_none_or(|(id, updated)| (&gist.updated_at, &gist.id) > (updated, id))
                {
                    newest = Some((gist.id, gist.updated_at));
                }
            }
            if is_last_page {
                return Ok(newest);
            }
            page += 1;
        }
    }

    pub async fn create_gist(&self, payload_json: &str) -> AppResult<(String, String)> {
        let res = self
            .request(reqwest::Method::POST, format!("{GITHUB_API_BASE}/gists"))
            .json(&build_gist_body(payload_json))
            .send()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

        let parsed = Self::parse_gist_response(res).await?;
        Ok((parsed.id, parsed.updated_at))
    }

    pub async fn update_gist(&self, gist_id: &str, payload_json: &str) -> AppResult<String> {
        let res = self
            .request(reqwest::Method::PATCH, format!("{GITHUB_API_BASE}/gists/{gist_id}"))
            .json(&build_gist_body(payload_json))
            .send()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

        let parsed = Self::parse_gist_response(res).await?;
        Ok(parsed.updated_at)
    }

    pub async fn fetch_gist(&self, gist_id: &str) -> AppResult<(String, String)> {
        let res = self
            .request(reqwest::Method::GET, format!("{GITHUB_API_BASE}/gists/{gist_id}"))
            .send()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

        let parsed = Self::parse_gist_response(res).await?;
        let content = parsed
            .files
            .get(SYNC_GIST_FILENAME)
            .and_then(|file| file.content.clone())
            .ok_or_else(|| AppError::NotFound(format!("sync gist {gist_id} is missing {SYNC_GIST_FILENAME}")))?;
        Ok((parsed.updated_at, content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn 기존_동기화_gist를_페이지_끝까지_검색하고_저장된_id를_우선한다() {
        use axum::{http::Uri, routing::get, Router};
        let router = Router::new().route(
            "/gists",
            get(|uri: Uri| async move {
                let query = uri.query().unwrap();
                assert!(query.split('&').any(|pair| pair == "per_page=100"));
                let gist = |id: &str, date: &str, description: &str| {
                    serde_json::json!({
                        "id": id, "updated_at": date, "description": description,
                        "files": { SYNC_GIST_FILENAME: {} }
                    })
                };
                let page = if query.split('&').any(|pair| pair == "page=1") {
                    let mut page = vec![gist("unrelated", "2026-09-23T00:00:00Z", "other"); GIST_PAGE_SIZE];
                    page[0] = gist("latest", "2026-09-22T00:00:00Z", SYNC_GIST_DESCRIPTION);
                    page
                } else {
                    vec![gist("preferred", "2026-09-21T00:00:00Z", SYNC_GIST_DESCRIPTION)]
                };
                (
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    serde_json::to_string(&page).unwrap(),
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::new();
        let gist = GistClient {
            client: &client,
            token: "test-token",
        };
        let latest = gist.discover_sync_gist_at(&base, None).await.unwrap().unwrap();
        assert_eq!(latest.0, "latest");
        let preferred = gist.discover_sync_gist_at(&base, Some("preferred")).await.unwrap().unwrap();
        assert_eq!(preferred.0, "preferred");
        task.abort();
    }

    #[tokio::test]
    async fn 빈_gist_목록과_인증_실패를_구별한다() {
        use axum::{routing::get, Router};
        let router = Router::new()
            .route("/empty/gists", get(|| async { "[]" }))
            .route("/denied/gists", get(|| async { axum::http::StatusCode::UNAUTHORIZED }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::new();
        let gist = GistClient {
            client: &client,
            token: "test-token",
        };
        assert!(gist.discover_sync_gist_at(&format!("{base}/empty"), None).await.unwrap().is_none());
        assert!(gist.discover_sync_gist_at(&format!("{base}/denied"), None).await.is_err());
        task.abort();
    }

    #[test]
    fn gist_바디는_비공개이며_지정된_파일명으로_담긴다() {
        let body = build_gist_body(r#"{"schemaVersion":1}"#);

        assert!(!body.public);
        assert_eq!(body.description, SYNC_GIST_DESCRIPTION);
        let file = body.files.get(SYNC_GIST_FILENAME).expect("file present");
        assert_eq!(file.content, r#"{"schemaVersion":1}"#);
    }

    #[test]
    fn gist_바디는_직렬화시_files_아래에_파일명_키를_가진다() {
        let body = build_gist_body("{}");
        let json = serde_json::to_value(&body).expect("serialize");

        assert_eq!(json["public"], false);
        assert!(json["files"].get(SYNC_GIST_FILENAME).is_some());
        assert_eq!(json["files"][SYNC_GIST_FILENAME]["content"], "{}");
    }
}
