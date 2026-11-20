use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn GoalsPage() -> Element {
    rsx! {
        PageHeader { title: "Ziele" }
        PageBody {
            Empty { EmptyTitle { "Ziele" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
