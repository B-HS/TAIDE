use taide_model::error::AppResult;
use taide_sync::github::GistHttpClient;

use crate::sync_actions::SyncGistPort;

pub use taide_sync::github::GistHttpClient as SyncGistHttpPort;

impl SyncGistPort for SyncGistHttpPort {
    async fn discover_sync_gist(&self, token: &str, preferred_id: Option<&str>) -> AppResult<Option<(String, String)>> {
        GistHttpClient::discover_sync_gist(self, token, preferred_id).await
    }

    async fn create_gist(&self, token: &str, payload_json: &str) -> AppResult<(String, String)> {
        GistHttpClient::create_gist(self, token, payload_json).await
    }

    async fn update_gist(&self, token: &str, gist_id: &str, payload_json: &str) -> AppResult<String> {
        GistHttpClient::update_gist(self, token, gist_id, payload_json).await
    }

    async fn fetch_gist(&self, token: &str, gist_id: &str) -> AppResult<(String, String)> {
        GistHttpClient::fetch_gist(self, token, gist_id).await
    }
}
