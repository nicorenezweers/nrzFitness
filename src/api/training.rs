use chrono::NaiveDate;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{
    EntryInput, Exercise, ExerciseEntry, ExerciseInput, Library, PersonalRecord, ProgressPoint, TemplateItemInput,
    TrainingHistory, WorkoutTemplate,
};
#[cfg(feature = "server")]
use crate::server::{db, pool};

// --- Übungen ---

#[server]
pub async fn get_library(pid: String) -> Result<Library, ServerFnError> {
    db::training::library(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn create_exercise(pid: String, input: ExerciseInput) -> Result<Exercise, ServerFnError> {
    db::training::create_exercise(pool(), &pid, input).await.map_err(e)
}

#[server]
pub async fn update_exercise(pid: String, id: String, input: ExerciseInput) -> Result<Exercise, ServerFnError> {
    db::training::update_exercise(pool(), &pid, &id, input).await.map_err(e)
}

#[server]
pub async fn delete_exercise(pid: String, id: String) -> Result<(), ServerFnError> {
    db::training::delete_exercise(pool(), &pid, &id).await.map_err(e)
}

// --- Einträge ---

#[server]
pub async fn get_training_day(pid: String, date: NaiveDate) -> Result<Vec<ExerciseEntry>, ServerFnError> {
    db::training::entries(pool(), &pid, date).await.map_err(e)
}

#[server]
pub async fn last_exercise_entry(
    pid: String,
    exercise_id: String,
    exclude: Option<String>,
) -> Result<Option<ExerciseEntry>, ServerFnError> {
    db::training::last_entry(pool(), &pid, &exercise_id, exclude).await.map_err(e)
}

#[server]
pub async fn save_exercise_entry(
    pid: String,
    id: Option<String>,
    input: EntryInput,
) -> Result<ExerciseEntry, ServerFnError> {
    db::training::save_entry(pool(), &pid, id, input).await.map_err(e)
}

#[server]
pub async fn delete_exercise_entry(pid: String, id: String) -> Result<(), ServerFnError> {
    db::training::delete_entry(pool(), &pid, &id).await.map_err(e)
}

#[server]
pub async fn set_exercise_set_done(pid: String, set_id: String, done: bool) -> Result<(), ServerFnError> {
    db::training::set_done(pool(), &pid, &set_id, done).await.map_err(e)
}

// --- Vorlagen ---

#[server]
pub async fn list_workout_templates(pid: String) -> Result<Vec<WorkoutTemplate>, ServerFnError> {
    db::training::templates(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn save_workout_template(
    pid: String,
    id: Option<String>,
    name: String,
    notes: Option<String>,
    items: Vec<TemplateItemInput>,
) -> Result<WorkoutTemplate, ServerFnError> {
    db::training::save_template(pool(), &pid, id, name, notes, items).await.map_err(e)
}

#[server]
pub async fn delete_workout_template(pid: String, id: String) -> Result<(), ServerFnError> {
    db::training::delete_template(pool(), &pid, &id).await.map_err(e)
}

#[server]
pub async fn apply_workout_template(pid: String, id: String, date: NaiveDate) -> Result<usize, ServerFnError> {
    db::training::apply_template(pool(), &pid, &id, date).await.map_err(e)
}

#[server]
pub async fn template_from_day(pid: String, date: NaiveDate, name: String) -> Result<WorkoutTemplate, ServerFnError> {
    db::training::template_from_day(pool(), &pid, date, name).await.map_err(e)
}

// --- Verlauf ---

#[server]
pub async fn get_training_history(pid: String, today: NaiveDate, weeks: i64) -> Result<TrainingHistory, ServerFnError> {
    db::training::history(pool(), &pid, today, weeks).await.map_err(e)
}

#[server]
pub async fn get_personal_records(pid: String) -> Result<Vec<PersonalRecord>, ServerFnError> {
    db::training::records(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn get_exercise_progress(pid: String, exercise_id: String) -> Result<Vec<ProgressPoint>, ServerFnError> {
    db::training::progress(pool(), &pid, &exercise_id).await.map_err(e)
}
