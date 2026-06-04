use std::time::Duration;

use fluxer_neptunium::model::id::{
    Id,
    marker::{GuildMarker, UserMarker},
};

pub const WORKER_API_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum WorkerToApiMessage {
    HeartbeatReq,
    HeartbeatRes,
    InvalidateCachedGuildPermissions(Id<GuildMarker>),
    InvalidateCachedGuildPermissionsForUser(Id<GuildMarker>, Id<UserMarker>),
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "t", content = "c")]
pub enum ApiToWorkerMessage {
    HeartbeatReq,
    HeartbeatRes,
    InvalidateCachedGuildPrefixes(Id<GuildMarker>),
}

#[repr(u16)]
pub enum WorkerApiWsCloseCode {
    HeartbeatTimeout = 4000,
}

impl WorkerApiWsCloseCode {
    pub fn from_u16(code: u16) -> Option<Self> {
        Some(match code {
            4000 => Self::HeartbeatTimeout,
            _ => return None,
        })
    }
}
