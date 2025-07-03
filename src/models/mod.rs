use std::{collections::HashMap, sync::Arc};

use mcp_server_rs::core::protocol::message::JsonRpcMessage;
use serde::Deserialize;
use tokio::sync::{RwLock, mpsc};

pub type SessionId = Arc<str>;
pub type SseSender = mpsc::UnboundedSender<JsonRpcMessage>;

#[derive(Clone, Default)]
pub struct App {
    pub channels: Arc<RwLock<HashMap<SessionId, SseSender>>>,
}

pub fn session_id() -> SessionId {
    Arc::from(format!("{:016x}", rand::random::<u128>()))
}

#[derive(Debug, Deserialize)]
pub struct SseQuery {
    pub service: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostQuery {
    pub session_id: String,
}
