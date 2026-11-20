use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn TrainingPage() -> Element {
    rsx! {
        PageHeader { title: "Training" }
        PageBody {
            Empty { EmptyTitle { "Training" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
