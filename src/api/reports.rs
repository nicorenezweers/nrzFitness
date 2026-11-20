use chrono::NaiveDate;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{ImportResult, Report};
#[cfg(feature = "server")]
use crate::server::{db, pool};

/// Bericht für den Zeitraum `from..=to` (inkl. Vergleich mit dem Zeitraum davor).
#[server]
pub async fn get_report(pid: String, from: NaiveDate, to: NaiveDate) -> Result<Report, ServerFnError> {
    db::reports::report(pool(), &pid, from, to, crate::util::today()).await.map_err(e)
}

/// JSON-Sicherung des Profils.
#[server]
pub async fn export_backup(pid: String) -> Result<String, ServerFnError> {
    db::reports::export_json(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn export_diary_csv(pid: String) -> Result<String, ServerFnError> {
    db::reports::diary_csv(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn export_checkins_csv(pid: String) -> Result<String, ServerFnError> {
    db::reports::checkins_csv(pool(), &pid).await.map_err(e)
}

/// Zwischenspeicher für Sicherungen, die in Teilen hochgeladen werden (Request-Grössenlimit).
#[cfg(feature = "server")]
type Uploads = std::collections::HashMap<String, Vec<Option<String>>>;

#[cfg(feature = "server")]
fn uploads() -> &'static std::sync::Mutex<Uploads> {
    static U: std::sync::OnceLock<std::sync::Mutex<Uploads>> = std::sync::OnceLock::new();
    U.get_or_init(Default::default)
}

/// Grösse eines Upload-Teils (Zeichen).
#[allow(dead_code)]
pub const IMPORT_CHUNK: usize = 512 * 1024;

/// Sicherung importieren: Teil `index` von `total`. Beim letzten Teil wird importiert (eine Transaktion).
#[server]
pub async fn import_backup_chunk(
    pid: String,
    upload: String,
    index: u32,
    total: u32,
    data: String,
) -> Result<ImportResult, ServerFnError> {
    if total == 0 || total > 2000 || index >= total {
        return Err(ServerFnError::new("Ungültiger Upload"));
    }
    let complete = {
        let mut map = uploads().lock().map_err(|_| ServerFnError::new("Upload fehlgeschlagen"))?;
        if !map.contains_key(&upload) && map.len() >= 8 {
            map.clear();
        }
        let parts = map.entry(upload.clone()).or_insert_with(|| vec![None; total as usize]);
        if parts.len() != total as usize {
            map.remove(&upload);
            return Err(ServerFnError::new("Ungültiger Upload"));
        }
        parts[index as usize] = Some(data);
        if parts.iter().all(Option::is_some) {
            map.remove(&upload).map(|p| p.into_iter().flatten().collect::<String>())
        } else {
            None
        }
    };
    match complete {
        Some(json) => db::reports::import_json(pool(), &pid, &json).await.map_err(e),
        None => Ok(ImportResult::default()),
    }
}
