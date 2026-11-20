use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn FastingPage() -> Element {
    rsx! {
        PageHeader { title: "Fasten" }
        PageBody {
            Empty { EmptyTitle { "Fasten" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
