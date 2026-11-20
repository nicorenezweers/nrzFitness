use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn ReportsPage() -> Element {
    rsx! {
        PageHeader { title: "Berichte" }
        PageBody {
            Empty { EmptyTitle { "Berichte" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
