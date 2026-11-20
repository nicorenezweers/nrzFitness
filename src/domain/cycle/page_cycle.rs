use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn CyclePage() -> Element {
    rsx! {
        PageHeader { title: "Zyklus" }
        PageBody {
            Empty { EmptyTitle { "Zyklus" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
