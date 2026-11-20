use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn SettingsPage() -> Element {
    rsx! {
        PageHeader { title: "Einstellungen" }
        PageBody {
            Empty { EmptyTitle { "Einstellungen" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
