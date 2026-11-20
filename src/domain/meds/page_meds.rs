use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn MedsPage() -> Element {
    rsx! {
        PageHeader { title: "Medikamente" }
        PageBody {
            Empty { EmptyTitle { "Medikamente" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
