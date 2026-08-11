use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::ai::providers::mask_provider_error;
use crate::domain::sync::types::{SYNC_GIST_DESCRIPTION, SYNC_GIST_FILENAME};
use crate::error::{AppError, AppResult};

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const USER_AGENT: &str = "TAIDE-Sync";

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

    pub async fn verify_token(&self) -> AppResult<()> {
        let res = self
            .request(reqwest::Method::GET, format!("{GITHUB_API_BASE}/gists"))
            .send()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

        if !res.status().is_success() {
            return Err(AppError::InvalidArgument("GitHub personal access token was rejected".to_string()));
        }
        Ok(())
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
