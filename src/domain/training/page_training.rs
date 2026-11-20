//! Training: Tagesansicht, Verlauf und Übungsbibliothek.

use dioxus::prelude::*;

use super::day::DayView;
use super::history::HistoryView;
use super::library::LibraryView;
use super::rest_timer::RestTimer;
use crate::api::training::get_library;
use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::segmented::Segmented;
use crate::model::Library;
use crate::state::use_app;

/// Gemeinsamer Zustand der Trainings-Seite (Bibliothek, Ruhe-Timer).
#[derive(Clone, Copy)]
pub struct TrainingCtx {
    pub library: Resource<Result<Library, ServerFnError>>,
    /// Ende der laufenden Pause (ms seit Epoche).
    pub rest_end: Signal<Option<i64>>,
    /// Pausenlänge in Sekunden (wird im Browser gemerkt).
    pub rest_len: Signal<i64>,
}

impl TrainingCtx {
    /// Bibliothek (leer, solange sie lädt).
    pub fn lib(&self) -> Library {
        self.library.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default()
    }

    pub fn start_rest(&mut self) {
        let len = *self.rest_len.peek();
        self.rest_end.set(Some(now_ms() + len * 1000));
    }

    /// Pause um `delta` Sekunden verlängern/verkürzen und als neue Standardlänge merken.
    pub fn adjust_rest(&mut self, delta: i64) {
        let len = (*self.rest_len.peek() + delta).clamp(15, 600);
        self.rest_len.set(len);
        let _ = document::eval(&format!("localStorage.setItem('nrzfitness.rest', '{len}');"));
        let end = *self.rest_end.peek();
        if let Some(end) = end {
            self.rest_end.set(Some((end + delta * 1000).max(now_ms())));
        }
    }
}

pub fn use_training() -> TrainingCtx {
    use_context::<TrainingCtx>()
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[component]
pub fn TrainingPage() -> Element {
    let app = use_app();
    let library = use_resource(move || async move {
        let _ = (app.refresh)();
        let pid = app.pid_tracked();
        get_library(pid).await
    });
    let mut ctx =
        use_context_provider(|| TrainingCtx { library, rest_end: Signal::new(None), rest_len: Signal::new(90) });
    use_effect(move || {
        spawn(async move {
            if let Ok(v) = document::eval("return Number(localStorage.getItem('nrzfitness.rest')) || 90;").await
                && let Some(n) = v.as_f64()
            {
                ctx.rest_len.set((n as i64).clamp(15, 600));
            }
        });
    });
    let mut tab = use_signal(|| "day".to_string());
    let tabs: Vec<(String, String)> = [("day", "Tag"), ("history", "Verlauf"), ("library", "Übungen")]
        .iter()
        .map(|(k, l)| (k.to_string(), l.to_string()))
        .collect();

    rsx! {
        PageHeader { title: "Training" }
        PageBody {
            Segmented { options: tabs, value: tab(), onchange: move |v| tab.set(v), class: "sm:max-w-sm" }
            {match tab().as_str() {
                "history" => rsx! { HistoryView {} },
                "library" => rsx! { LibraryView {} },
                _ => rsx! { DayView {} },
            }}
        }
        RestTimer {}
    }
}
