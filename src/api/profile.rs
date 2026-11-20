use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{Goals, Profile, ProfileInput};
#[cfg(feature = "server")]
use crate::server::{db, pool};

#[server]
pub async fn list_profiles() -> Result<Vec<Profile>, ServerFnError> {
    db::profile::list(pool()).await.map_err(e)
}

#[server]
pub async fn create_profile(input: ProfileInput) -> Result<Profile, ServerFnError> {
    db::profile::create(pool(), input).await.map_err(e)
}

#[server]
pub async fn update_profile(id: String, input: ProfileInput) -> Result<Profile, ServerFnError> {
    db::profile::update(pool(), &id, input).await.map_err(e)
}

#[server]
pub async fn delete_profile(id: String) -> Result<(), ServerFnError> {
    db::profile::delete(pool(), &id).await.map_err(e)
}

#[server]
pub async fn get_goals(pid: String) -> Result<Goals, ServerFnError> {
    db::profile::goals(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn save_goals(pid: String, goals: Goals) -> Result<Goals, ServerFnError> {
    db::profile::save_goals(pool(), &pid, goals).await.map_err(e)
}

#[server]
pub async fn latest_weight(pid: String) -> Result<Option<f64>, ServerFnError> {
    db::profile::latest_weight(pool(), &pid).await.map_err(e)
}

/// Erstes Einrichten: Profil anlegen, Startgewicht speichern und Ziele vorschlagen.
#[server]
pub async fn onboard(input: ProfileInput, weight: Option<f64>, weekly_change: f64, target_weight: Option<f64>) -> Result<Profile, ServerFnError> {
    let p = db::profile::create(pool(), input).await.map_err(e)?;
    let base = db::profile::goals(pool(), &p.id).await.map_err(e)?;
    if let Some(w) = weight.filter(|w| *w > 0.0) {
        db::profile::set_weight(pool(), &p.id, crate::util::today(), w).await.map_err(e)?;
        let mut g = crate::model::suggested_goals(&p, w, weekly_change, &base);
        g.target_weight = target_weight;
        db::profile::save_goals(pool(), &p.id, g).await.map_err(e)?;
    }
    Ok(p)
}
