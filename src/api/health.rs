use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::HealthStatus;
#[cfg(feature = "server")]
use crate::server::{health, pool};

#[server]
pub async fn health_status(pid: String) -> Result<HealthStatus, ServerFnError> {
    health::status(pool(), &pid).await.map_err(e)
}

/// Verbindung für Automationen erstellen/erneuern (`enable = false` trennt sie).
#[server]
pub async fn health_set_token(pid: String, enable: bool) -> Result<Option<String>, ServerFnError> {
    health::set_token(pool(), &pid, enable).await.map_err(e)
}
