use std::sync::Arc;

use futures_util::future::BoxFuture;
use serde_json::Value;
use tauri::ipc::InvokeResponseBody;
use tauri::AppHandle;

pub type ChannelSink = Box<dyn Fn(InvokeResponseBody) -> tauri::Result<()> + Send + Sync>;
pub type ChannelFactory = Arc<dyn Fn(String) -> ChannelSink + Send + Sync>;
pub type RemoteJsonDispatch = fn(AppHandle, String, Value, ChannelFactory) -> BoxFuture<'static, Result<String, Value>>;
pub type RemoteRawDispatch = fn(AppHandle, String, Value) -> BoxFuture<'static, Result<Vec<u8>, Value>>;

/// Application-owned command dispatch operations used by remote WebSocket sessions.
pub struct RemoteDispatchPort {
    pub json: RemoteJsonDispatch,
    pub raw: RemoteRawDispatch,
}
